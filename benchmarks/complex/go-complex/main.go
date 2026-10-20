// Complex benchmark: three CPU-heavy GET routes via net/http/cgi + chi router.
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/cgi"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/go-chi/chi/v5"
)

var (
	processStart       = time.Now()
	untilProcessAtMain *uint64
)

const (
	defaultPrimeLimit = 400_000
	defaultFibN       = 42
	defaultMatrixSize = 128
	maxPrimeLimit     = 1_000_000
	maxFibN           = 50
	maxMatrixSize     = 400
	saltModPrime      = 5_000
	saltModMatrix     = 16
	fibSeed           = 42
	fibRepeatBase     = 3_000
	fibRepeatSaltMod  = 7_000
)

type phaseProfile struct {
	UntilProcessUs *uint64 `json:"until_process_us,omitempty"`
	UntilComputeUs *uint64 `json:"until_compute_us,omitempty"`
	StartupUs      uint64  `json:"startup_us"`
	HandlerSetupUs uint64  `json:"handler_setup_us"`
	ComputeUs      uint64  `json:"compute_us"`
	PostComputeUs  uint64  `json:"post_compute_us"`
	TotalUs        uint64  `json:"total_us"`
}

type computeResponse struct {
	Route   string        `json:"route"`
	Salt    uint64        `json:"salt"`
	Result  interface{}   `json:"result"`
	Profile *phaseProfile `json:"profile,omitempty"`
}

func benchSentUs() (uint64, bool) {
	s := os.Getenv("HTTP_X_BENCH_SENT_US")
	if s == "" {
		return 0, false
	}
	v, err := strconv.ParseUint(s, 10, 64)
	return v, err == nil
}

func sinceSentUs(sent uint64) uint64 {
	return uint64(time.Now().UnixMicro()) - sent
}

func logBenchPreCompute(route string) {
	if os.Getenv("BENCH_TIMING") != "1" {
		return
	}
	f, err := os.OpenFile("/var/log/bench-timing/requests.log", os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o666)
	if err != nil {
		return
	}
	_, _ = fmt.Fprintf(f, "go-complex\t%s\t%d\n", route, elapsedUs())
	_ = f.Close()
}

func elapsedUs() uint64 {
	return uint64(time.Since(processStart).Microseconds())
}

func logPhase(profile bool, phase string) {
	if !profile {
		return
	}
	fmt.Fprintf(os.Stderr, "bench-timing lang=go-complex phase=%s elapsed_us=%d\n", phase, elapsedUs())
}

func wantsProfile(r *http.Request) bool {
	return r.URL.Query().Get("profile") == "1"
}

func requestSalt(r *http.Request) uint64 {
	if s := r.URL.Query().Get("salt"); s != "" {
		if v, err := strconv.ParseUint(s, 10, 64); err == nil {
			return v
		}
	}
	return uint64(time.Now().UnixNano())
}

func effectivePrimeLimit(base, max int, salt uint64) int {
	v := base + int(salt%saltModPrime)
	if v > max {
		return max
	}
	return v
}

func fibonacciWork(salt uint64) uint64 {
	repeats := fibRepeatBase + uint32(salt%fibRepeatSaltMod)
	var acc uint64
	for i := uint32(0); i < repeats; i++ {
		acc += fibonacci(fibSeed)
	}
	return acc
}

func effectiveMatrixSize(base, max int, salt uint64) int {
	v := base + int(salt%saltModMatrix)
	if v > max {
		return max
	}
	return v
}

func primeCount(limit int) int64 {
	if limit <= 2 {
		return 0
	}
	composite := make([]bool, limit)
	var count int64
	for i := 2; i < limit; i++ {
		if composite[i] {
			continue
		}
		count++
		for j := i * i; j < limit; j += i {
			composite[j] = true
		}
	}
	return count
}

func fibonacci(n uint32) uint64 {
	if n == 0 {
		return 0
	}
	a, b := uint64(0), uint64(1)
	for i := uint32(1); i < n; i++ {
		a, b = b, a+b
	}
	return b
}

func matrixChecksum(n int) float64 {
	if n == 0 {
		return 0
	}
	size := n * n
	a := make([]float64, size)
	b := make([]float64, size)
	c := make([]float64, size)
	for i := 0; i < size; i++ {
		a[i] = float64(i) * 0.001
		b[i] = float64(i) * 0.002
	}
	for i := 0; i < n; i++ {
		for j := 0; j < n; j++ {
			sum := 0.0
			for k := 0; k < n; k++ {
				sum += a[i*n+k] * b[k*n+j]
			}
			c[i*n+j] = sum
		}
	}
	total := 0.0
	for _, v := range c {
		total += v
	}
	return total
}

func writeJSON(w http.ResponseWriter, route string, salt uint64, result interface{}, profile *phaseProfile) {
	w.Header().Set("Cache-Control", "no-store, no-cache, must-revalidate")
	w.Header().Set("Pragma", "no-cache")
	w.Header().Set("Content-Type", "application/json")

	_ = json.NewEncoder(w).Encode(computeResponse{Route: route, Salt: salt, Result: result, Profile: profile})
}

func runTimed(w http.ResponseWriter, route string, salt uint64, profileOn bool, compute func() interface{}) {
	handlerEnter := elapsedUs()
	logPhase(profileOn, "handler_enter")
	preCompute := elapsedUs()
	logBenchPreCompute(route)
	var untilCompute *uint64
	if sent, ok := benchSentUs(); ok {
		v := sinceSentUs(sent)
		untilCompute = &v
	}
	logPhase(profileOn, "pre_compute")
	start := time.Now()
	result := compute()
	computeUs := uint64(time.Since(start).Microseconds())
	logPhase(profileOn, "post_compute")
	total := elapsedUs()
	postCompute := total - preCompute - computeUs

	var prof *phaseProfile
	if profileOn {
		prof = &phaseProfile{
			UntilProcessUs: untilProcessAtMain,
			UntilComputeUs: untilCompute,
			StartupUs:      handlerEnter,
			HandlerSetupUs: preCompute - handlerEnter,
			ComputeUs:      computeUs,
			PostComputeUs:  postCompute,
			TotalUs:        total,
		}
		fmt.Fprintf(os.Stderr, "bench-timing lang=go-complex summary startup_us=%d compute_us=%d total_us=%d\n",
			prof.StartupUs, prof.ComputeUs, prof.TotalUs)
	}
	writeJSON(w, route, salt, result, prof)
}

// cgiPath makes PATH_INFO (or SCRIPT_NAME-stripped path) visible to chi as r.URL.Path.
func cgiPath(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if pi := os.Getenv("PATH_INFO"); pi != "" {
			r.URL.Path = pi
		} else {
			path := strings.Trim(r.URL.Path, "/")
			if script := strings.Trim(os.Getenv("SCRIPT_NAME"), "/"); script != "" {
				if strings.HasPrefix(path, script) {
					path = strings.TrimPrefix(path, script)
					path = strings.TrimPrefix(path, "/")
				}
			}
			if path == "" {
				r.URL.Path = "/"
			} else {
				r.URL.Path = "/" + path
			}
		}
		next.ServeHTTP(w, r)
	})
}

func pathUint(r *http.Request, name string, fallback int) int {
	s := chi.URLParam(r, name)
	if s == "" {
		return fallback
	}
	v, err := strconv.Atoi(s)
	if err != nil {
		return fallback
	}
	return v
}

func routePrimes(w http.ResponseWriter, r *http.Request) {
	limit := pathUint(r, "limit", defaultPrimeLimit)
	salt := requestSalt(r)
	limit = effectivePrimeLimit(limit, maxPrimeLimit, salt)
	runTimed(w, "primes", salt, wantsProfile(r), func() interface{} { return primeCount(limit) })
}

func routePrimesDefault(w http.ResponseWriter, r *http.Request) {
	routePrimes(w, r)
}

func routeFibonacci(w http.ResponseWriter, r *http.Request) {
	salt := requestSalt(r)
	runTimed(w, "fibonacci", salt, wantsProfile(r), func() interface{} { return fibonacciWork(salt) })
}

func routeFibonacciDefault(w http.ResponseWriter, r *http.Request) {
	routeFibonacci(w, r)
}

func routeMatrix(w http.ResponseWriter, r *http.Request) {
	size := pathUint(r, "size", defaultMatrixSize)
	salt := requestSalt(r)
	size = effectiveMatrixSize(size, maxMatrixSize, salt)
	runTimed(w, "matrix", salt, wantsProfile(r), func() interface{} {
		return map[string]interface{}{"size": size, "checksum": matrixChecksum(size)}
	})
}

func routeMatrixDefault(w http.ResponseWriter, r *http.Request) {
	routeMatrix(w, r)
}

func notFound(w http.ResponseWriter, _ *http.Request) {
	w.Header().Set("Cache-Control", "no-store")
	http.Error(w, "routes: /primes/{limit}, /fibonacci/{n}, /matrix/{size}\n", http.StatusNotFound)
}

func newRouter() http.Handler {
	r := chi.NewRouter()
	r.Get("/primes/{limit}", routePrimes)
	r.Get("/primes", routePrimesDefault)
	r.Get("/fibonacci/{n}", routeFibonacci)
	r.Get("/fibonacci", routeFibonacciDefault)
	r.Get("/matrix/{size}", routeMatrix)
	r.Get("/matrix", routeMatrixDefault)
	r.NotFound(notFound)
	return cgiPath(r)
}

func parseRawSalt(query string) (uint64, bool) {
	for _, pair := range strings.Split(query, "&") {
		k, v, ok := strings.Cut(pair, "=")
		if !ok || k != "salt" {
			continue
		}
		parsed, err := strconv.ParseUint(v, 10, 64)
		return parsed, err == nil
	}
	return 0, false
}

func rawRequestSalt() uint64 {
	if salt, ok := parseRawSalt(os.Getenv("QUERY_STRING")); ok {
		return salt
	}
	return uint64(time.Now().UnixNano())
}

func rawRouteAndArg() (string, string) {
	path := strings.TrimSpace(os.Getenv("PATH_INFO"))
	path = strings.TrimPrefix(path, "/")
	route, arg, ok := strings.Cut(path, "/")
	if !ok {
		return route, ""
	}
	return route, arg
}

func rawPathInt(arg string, fallback int) int {
	if arg == "" {
		return fallback
	}
	v, err := strconv.Atoi(arg)
	if err != nil {
		return fallback
	}
	return v
}

func writeRawJSON(route string, salt uint64, result interface{}) error {
	body, err := json.Marshal(computeResponse{Route: route, Salt: salt, Result: result})
	if err != nil {
		return err
	}
	var out bytes.Buffer
	out.WriteString("Status: 200 OK\n")
	out.WriteString("Cache-Control: no-store, no-cache, must-revalidate\n")
	out.WriteString("Pragma: no-cache\n")
	out.WriteString("Content-Type: application/json\n\n")
	out.Write(body)
	_, err = os.Stdout.Write(out.Bytes())
	return err
}

func writeRawStatus(status int, reason string, body string) error {
	_, _ = fmt.Fprintf(os.Stdout, "Status: %d %s\n", status, reason)
	_, _ = os.Stdout.Write([]byte("Cache-Control: no-store\n\n"))
	_, err := os.Stdout.Write([]byte(body))
	return err
}

func serveRawCGI() error {
	if os.Getenv("REQUEST_METHOD") != "GET" {
		return writeRawStatus(http.StatusMethodNotAllowed, "Method Not Allowed", "Method not allowed\n")
	}

	route, arg := rawRouteAndArg()
	salt := rawRequestSalt()
	switch route {
	case "primes":
		limit := rawPathInt(arg, defaultPrimeLimit)
		limit = effectivePrimeLimit(limit, maxPrimeLimit, salt)
		return writeRawJSON("primes", salt, primeCount(limit))
	case "fibonacci":
		return writeRawJSON("fibonacci", salt, fibonacciWork(salt))
	case "matrix":
		size := rawPathInt(arg, defaultMatrixSize)
		size = effectiveMatrixSize(size, maxMatrixSize, salt)
		return writeRawJSON("matrix", salt, map[string]interface{}{"size": size, "checksum": matrixChecksum(size)})
	default:
		return writeRawStatus(http.StatusNotFound, "Not Found", "routes: /primes/{limit}, /fibonacci/{n}, /matrix/{size}\n")
	}
}

func main() {
	if os.Getenv("GO_COMPLEX_RAW_CGI") == "1" {
		if err := serveRawCGI(); err != nil {
			fmt.Fprintf(os.Stderr, "raw CGI error: %v\n", err)
			os.Exit(1)
		}
		return
	}

	if sent, ok := benchSentUs(); ok {
		v := sinceSentUs(sent)
		untilProcessAtMain = &v
	}
	logPhase(strings.Contains(os.Getenv("QUERY_STRING"), "profile=1"), "pre_serve_cgi")
	if err := cgi.Serve(newRouter()); err != nil {
		panic(err)
	}
}
