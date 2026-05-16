<?php
/**
 * Complex benchmark: three CPU-heavy GET routes (mod_php).
 *
 * Per-request ?salt= or time-based salt — effective workload changes every request.
 */
declare(strict_types=1);

$processStartNs = hrtime(true);
/** @var int|null */
$untilProcessUs = null;

const DEFAULT_PRIME_LIMIT = 400_000;
const DEFAULT_FIB_N = 42;
const DEFAULT_MATRIX_SIZE = 128;
const MAX_PRIME_LIMIT = 1_000_000;
const MAX_FIB_N = 50;
const MAX_MATRIX_SIZE = 200;
const SALT_MOD_PRIME = 5_000;
const SALT_MOD_MATRIX = 16;
const FIB_SEED = 42;
const FIB_REPEAT_BASE = 3_000;
const FIB_REPEAT_SALT_MOD = 7_000;

function request_salt(?int $explicit): int
{
    if ($explicit !== null) {
        return $explicit;
    }
    return (int) (microtime(true) * 1_000_000) % (2 ** 30);
}

function effective_prime_limit(int $base, int $salt): int
{
    return min($base + ($salt % SALT_MOD_PRIME), MAX_PRIME_LIMIT);
}

function fibonacci_work(int $salt): int
{
    $repeats = FIB_REPEAT_BASE + ($salt % FIB_REPEAT_SALT_MOD);
    $acc = 0;
    for ($i = 0; $i < $repeats; $i++) {
        $acc = ($acc + fibonacci(FIB_SEED)) % (2 ** 62);
    }
    return $acc;
}

function effective_matrix_size(int $base, int $salt): int
{
    return min($base + ($salt % SALT_MOD_MATRIX), MAX_MATRIX_SIZE);
}

function prime_count(int $limit): int
{
    if ($limit <= 2) {
        return 0;
    }
    $sieve = array_fill(0, $limit, true);
    $sieve[0] = $sieve[1] = false;
    $count = 0;
    for ($i = 2; $i < $limit; $i++) {
        if (!$sieve[$i]) {
            continue;
        }
        $count++;
        for ($j = $i * $i; $j < $limit; $j += $i) {
            $sieve[$j] = false;
        }
    }
    return $count;
}

function fibonacci(int $n): int
{
    if ($n === 0) {
        return 0;
    }
    $a = 0;
    $b = 1;
    for ($i = 1; $i < $n; $i++) {
        [$a, $b] = [$b, $a + $b];
    }
    return $b;
}

function matrix_checksum(int $n): float
{
    if ($n === 0) {
        return 0.0;
    }
    $size = $n * $n;
    $a = [];
    $b = [];
    for ($i = 0; $i < $size; $i++) {
        $a[$i] = $i * 0.001;
        $b[$i] = $i * 0.002;
    }
    $c = array_fill(0, $size, 0.0);
    for ($i = 0; $i < $n; $i++) {
        for ($j = 0; $j < $n; $j++) {
            $sum = 0.0;
            for ($k = 0; $k < $n; $k++) {
                $sum += $a[$i * $n + $k] * $b[$k * $n + $j];
            }
            $c[$i * $n + $j] = $sum;
        }
    }
    return array_sum($c);
}

function bench_sent_us(): ?int
{
    $raw = $_SERVER['HTTP_X_BENCH_SENT_US'] ?? '';
    if ($raw === '' || !ctype_digit((string) $raw)) {
        return null;
    }
    return (int) $raw;
}

function since_sent_us(int $sent): int
{
    return (int) (microtime(true) * 1_000_000) - $sent;
}

function capture_until_process(): void
{
    global $untilProcessUs;
    $sent = bench_sent_us();
    if ($sent !== null) {
        $untilProcessUs = since_sent_us($sent);
    }
}

function elapsed_us(): int
{
    global $processStartNs;
    return (int) ((hrtime(true) - $processStartNs) / 1000);
}

function log_bench_pre_compute(string $route): void
{
    if (($_SERVER['BENCH_TIMING'] ?? '') !== '1') {
        return;
    }
    file_put_contents(
        '/var/log/bench-timing/requests.log',
        'php-complex' . "\t" . $route . "\t" . elapsed_us() . "\n",
        FILE_APPEND
    );
}

function bench_log(string $line): void
{
    // STDERR exists in CLI only; mod_php needs php://stderr.
    file_put_contents('php://stderr', $line);
}

function log_phase(bool $profile, string $phase): void
{
    if (!$profile) {
        return;
    }
    bench_log(sprintf("bench-timing lang=php-complex phase=%s elapsed_us=%d\n", $phase, elapsed_us()));
}

function wants_profile(): bool
{
    return isset($_GET['profile']) && (string) $_GET['profile'] === '1';
}

/** @return array{startup_us:int,handler_setup_us:int,compute_us:int,post_compute_us:int,total_us:int}|null */
function run_timed(string $route, int $salt, bool $profile, callable $compute): ?array
{
    $handlerEnter = elapsed_us();
    log_phase($profile, 'handler_enter');
    $preCompute = elapsed_us();
    log_bench_pre_compute($route);
    $untilCompute = null;
    $sent = bench_sent_us();
    if ($sent !== null) {
        $untilCompute = since_sent_us($sent);
    }
    log_phase($profile, 'pre_compute');
    $t0 = hrtime(true);
    $result = $compute();
    $computeUs = (int) ((hrtime(true) - $t0) / 1000);
    log_phase($profile, 'post_compute');
    $total = elapsed_us();
    $postCompute = $total - $preCompute - $computeUs;
    $prof = null;
    if ($profile) {
        global $untilProcessUs;
        $prof = [
            'until_process_us' => $untilProcessUs,
            'until_compute_us' => $untilCompute,
            'startup_us' => $handlerEnter,
            'handler_setup_us' => $preCompute - $handlerEnter,
            'compute_us' => $computeUs,
            'post_compute_us' => $postCompute,
            'total_us' => $total,
        ];
        bench_log(sprintf(
            "bench-timing lang=php-complex summary startup_us=%d compute_us=%d total_us=%d\n",
            $handlerEnter,
            $computeUs,
            $total
        ));
    }
    respond_json($route, $salt, $result, $prof);
    return $prof;
}

function respond_json(string $route, int $salt, mixed $result, ?array $profile = null): void
{
    header('Cache-Control: no-store, no-cache, must-revalidate');
    header('Pragma: no-cache');
    header('Content-Type: application/json; charset=utf-8');
    $payload = ['route' => $route, 'salt' => $salt, 'result' => $result];
    if ($profile !== null) {
        $payload['profile'] = $profile;
    }
    echo json_encode($payload, JSON_THROW_ON_ERROR);
}

$method = $_SERVER['REQUEST_METHOD'] ?? '';
if ($method !== 'GET') {
    http_response_code(405);
    header('Cache-Control: no-store');
    header('Content-Type: text/plain; charset=utf-8');
    echo "method not allowed\n";
    exit;
}

capture_until_process();

$explicitSalt = isset($_GET['salt']) && ctype_digit((string) $_GET['salt'])
    ? (int) $_GET['salt']
    : null;
$salt = request_salt($explicitSalt);

$path = $_SERVER['PATH_INFO'] ?? '';
$parts = array_values(array_filter(explode('/', trim($path, '/'))));
if ($parts === []) {
    http_response_code(404);
    header('Content-Type: text/plain; charset=utf-8');
    echo "routes: /primes/{limit}, /fibonacci/{n}, /matrix/{size}\n";
    exit;
}

$name = $parts[0];
$arg = isset($parts[1]) && ctype_digit($parts[1]) ? (int) $parts[1] : null;
$profile = wants_profile();
log_phase($profile, 'handler_enter');

match ($name) {
    'primes' => run_timed(
        'primes',
        $salt,
        $profile,
        fn () => prime_count(effective_prime_limit($arg ?? DEFAULT_PRIME_LIMIT, $salt))
    ),
    'fibonacci' => run_timed('fibonacci', $salt, $profile, fn () => fibonacci_work($salt)),
    'matrix' => run_timed('matrix', $salt, $profile, function () use ($arg, $salt): array {
        $size = effective_matrix_size($arg ?? DEFAULT_MATRIX_SIZE, $salt);
        return ['size' => $size, 'checksum' => matrix_checksum($size)];
    }),
    default => (function (): void {
        http_response_code(404);
        header('Content-Type: text/plain; charset=utf-8');
        echo "routes: /primes/{limit}, /fibonacci/{n}, /matrix/{size}\n";
    })(),
};
