// Benchmark go-webhook: Stripe-style webhook ack via net/http/cgi (stdlib only).
package main

import (
	"encoding/json"
	"net/http"
	"net/http/cgi"
)

const webhookSecret = "bench-secret"

type webhookEvent struct {
	ID   string `json:"id"`
	Type string `json:"type"`
}

type webhookAck struct {
	Received bool   `json:"received"`
	ID       string `json:"id"`
}

func handleWebhook(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed\n", http.StatusMethodNotAllowed)
		return
	}
	if r.Header.Get("X-Webhook-Secret") != webhookSecret {
		http.Error(w, "unauthorized\n", http.StatusUnauthorized)
		return
	}

	var evt webhookEvent
	if err := json.NewDecoder(r.Body).Decode(&evt); err != nil {
		http.Error(w, "bad request\n", http.StatusBadRequest)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(webhookAck{Received: true, ID: evt.ID})
}

func main() {
	if err := cgi.Serve(http.HandlerFunc(handleWebhook)); err != nil {
		panic(err)
	}
}
