// Complex benchmark: three CPU-heavy GET routes via net/http/cgi.
// Go 1.22+ ServeMux with method-based patterns + path parameters —
// mirrors Rust's Axum routing in marty-complex.
package main

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/cgi"
	"os"
	"strconv"
	"strings"
	"time"
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
	maxMatrixSize     = 200
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

type computeResponse struct {
	Route   string        `json:"route"`
	Salt    uint64        `json:"salt"`
	Result  interface{}   `json:"result"`
	Profile *phaseProfile `json:"profile,omitempty"`
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

// stripCgiPrefix wraps an http.Handler, removing SCRIPT_NAME (or using PATH_INFO)
// from the request URL so that route patterns are relative to the CGI mount point.
//
// This mirrors what Rust's multi_mount_cgi_router_from_env does automatically:
// it lets you write routes like /primes/{limit} instead of manually stripping
// /cgi-bin/go-complex from the path.
func stripCgiPrefix(next http.Handler) http.Handler {
	scriptName := os.Getenv("SCRIPT_NAME")
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// CGI servers typically set PATH_INFO to the path after SCRIPT_NAME.
		if pi := os.Getenv("PATH_INFO"); pi != "" {
			r.URL.Path = pi
			r.RequestURI = pi
		} else if scriptName != "" {
			// Fallback: strip SCRIPT_NAME prefix manually.
			cleaned := strings.TrimPrefix(r.URL.Path, scriptName)
			if !strings.HasPrefix(cleaned, "/") {
				cleaned = "/" + cleaned
			}
			r.URL.Path = cleaned
			r.RequestURI = cleaned
		}
		next.ServeHTTP(w, r)
	})
}

// ---- Route handlers (mirror Rust's Axum route functions) ----

func routePrimes(w http.ResponseWriter, r *http.Request) {
	salt := requestSalt(r)
	profileOn := wantsProfile(r)

	limitStr := r.PathValue("limit")
	limit := defaultPrimeLimit
	if limitStr != "" {
		if v, err := strconv.Atoi(limitStr); err == nil {
			limit = v
		}
	}
	limit = effectivePrimeLimit(limit, maxPrimeLimit, salt)
	runTimed(w, "primes", salt, profileOn, func() interface{} { return primeCount(limit) })
}

func routeFibonacci(w http.ResponseWriter, r *http.Request) {
	salt := requestSalt(r)
	profileOn := wantsProfile(r)
	runTimed(w, "fibonacci", salt, profileOn, func() interface{} { return fibonacciWork(salt) })
}

func routeMatrix(w http.ResponseWriter, r *http.Request) {
	salt := requestSalt(r)
	profileOn := wantsProfile(r)

	sizeStr := r.PathValue("size")
	size := defaultMatrixSize
	if sizeStr != "" {
		if v, err := strconv.Atoi(sizeStr); err == nil {
			size = v
		}
	}
	size = effectiveMatrixSize(size, maxMatrixSize, salt)
	runTimed(w, "matrix", salt, profileOn, func() interface{} {
		return map[string]interface{}{"size": size, "checksum": matrixChecksum(size)}
	})
}

func notFound(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Cache-Control", "no-store")
	http.Error(w, "routes: /primes/{limit}, /fibonacci/{n}, /matrix/{size}\n", http.StatusNotFound)
}

func main() {
	if sent, ok := benchSentUs(); ok {
		v := sinceSentUs(sent)
		untilProcessAtMain = &v
	}

	// ---- Declarative routes, just like Rust's Axum Router ----
	//
	// Rust (marty-complex):
	//   Router::new()
	//     .route("/primes/{limit}", get(route_primes))
	//     .route("/primes",         get(route_primes_default))
	//     .route("/fibonacci/{n}",  get(route_fibonacci))
	//     ...
	//     .fallback(not_found);
	//
	// Go (this file):
	mux := http.NewServeMux()
	mux.HandleFunc("GET /primes/{limit}", routePrimes)
	mux.HandleFunc("GET /primes", routePrimes)
	mux.HandleFunc("GET /fibonacci/{n}", routeFibonacci)
	mux.HandleFunc("GET /fibonacci", routeFibonacci)
	mux.HandleFunc("GET /matrix/{size}", routeMatrix)
	mux.HandleFunc("GET /matrix", routeMatrix)
	mux.HandleFunc("/", notFound)

	// stripCgiPrefix removes /cgi-bin/go-complex from the URL path,
	// like Rust's multi_mount_cgi_router_from_env.
	handler := stripCgiPrefix(mux)

	logPhase(strings.Contains(os.Getenv("QUERY_STRING"), "profile=1"), "pre_serve_cgi")
	if err := cgi.Serve(handler); err != nil {
		panic(err)
	}
}
