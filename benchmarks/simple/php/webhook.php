<?php
/**
 * Benchmark php-webhook: mod_php (in-process Apache module, not CGI).
 *
 * Same contract as the other benchmarks/simple webhook implementations.
 */
declare(strict_types=1);

const WEBHOOK_SECRET = 'bench-secret';

if (($_SERVER['REQUEST_METHOD'] ?? '') !== 'POST') {
    http_response_code(405);
    header('Content-Type: text/plain; charset=utf-8');
    echo "method not allowed\n";
    exit;
}

if (($_SERVER['HTTP_X_WEBHOOK_SECRET'] ?? '') !== WEBHOOK_SECRET) {
    http_response_code(401);
    header('Content-Type: text/plain; charset=utf-8');
    echo "unauthorized\n";
    exit;
}

$raw = file_get_contents('php://input');
if ($raw === false) {
    http_response_code(400);
    header('Content-Type: text/plain; charset=utf-8');
    echo "bad request\n";
    exit;
}

/** @var mixed $evt */
$evt = json_decode($raw, true);
if (!is_array($evt) || !isset($evt['id']) || !is_string($evt['id'])) {
    http_response_code(400);
    header('Content-Type: text/plain; charset=utf-8');
    echo "bad request\n";
    exit;
}

header('Content-Type: application/json; charset=utf-8');
echo json_encode(['received' => true, 'id' => $evt['id']], JSON_THROW_ON_ERROR);
