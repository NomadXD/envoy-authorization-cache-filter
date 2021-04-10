package main

import (
	"encoding/json"
	"fmt"
	"net/http"
)

// CacheRecord represents a single record for a particular route
type CacheRecord struct {
	Path  string
	Quota int
	Used  int
}

// AuthRequest represents the data that is received from the cache filter
type AuthRequest struct {
	Path  string `json:"path"`
	Token string `json:"token"`
}

// AuthResponse represents the data that is sent as the auth response
type AuthResponse struct {
	Status          int    `json:"status"`
	Message         string `json:"message"`
	RateLimitHeader string `json:"ratelimit"`
}

func auth(w http.ResponseWriter, req *http.Request) {

	switch req.Method {
	case "POST":
		fmt.Println(req)
		var auth AuthRequest
		// Decode the JSON in the body and overwrite 'tom' with it
		err := json.NewDecoder(req.Body).Decode(&auth)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
		fmt.Println("ZXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX")
		fmt.Println(req.Body)
		fmt.Println(auth)
		fmt.Println(auth.Token)
		fmt.Println(auth.Path)
		if auth.Token == "AUTH" {
			authResponse := AuthResponse{200, "Success", "X-Ratelimit-Header"}
			jsonResponse, _ := json.Marshal(authResponse)
			w.Write(jsonResponse)
		} else {
			authResponse := AuthResponse{401, "Unauthorized", "X-Ratelimit-Header"}
			jsonResponse, _ := json.Marshal(authResponse)
			w.Write(jsonResponse)
		}
	default:
		w.WriteHeader(http.StatusMethodNotAllowed)
		fmt.Fprintf(w, "I can't do that.")
	}
}

func cache(w http.ResponseWriter, req *http.Request) {

	for name, headers := range req.Header {
		for _, h := range headers {
			fmt.Fprintf(w, "%v: %v\n", name, h)
		}
	}
}

func main() {

	cacheArr := []CacheRecord{{"/foo", 10, 0}, {"/bar", 10, 0}, {"/baz", 10, 0}}
	fmt.Println(cacheArr)
	http.HandleFunc("/auth", auth)
	http.HandleFunc("/cache", cache)

	http.ListenAndServe(":8000", nil)
}
