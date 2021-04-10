package main

import (
	"encoding/json"
	"fmt"
	"net/http"
)

// Cache represents a single record for a particular route
type Cache struct {
	FooPath  string `json:"foopath"`
	FooQuota int    `json:"fooquota"`
	FooUsed  int    `json:"fooused"`

	BarPath  string `json:"barpath"`
	BarQuota int    `json:"barquota"`
	BarUsed  int    `json:"barused"`

	BazPath  string `json:"bazpath"`
	BazQuota int    `json:"bazquota"`
	BazUsed  int    `json:"bazused"`
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

// type CacheMap struct {
// 	Records []CacheRecord `json:"array"`
// }

var cache Cache

func authHandler(w http.ResponseWriter, req *http.Request) {

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

func cacheHandler(w http.ResponseWriter, req *http.Request) {

	switch req.Method {
	case "POST":
		var cacheUpdate Cache
		//err := json.Unmarshal(req.Body, &temp)
		fmt.Println(req.Body)
		err := json.NewDecoder(req.Body).Decode(&cacheUpdate)
		if err != nil {
			fmt.Printf("Error : %v", err)
			//panic(err)
		}
		fmt.Printf("MMMMMMMMMMMMMMMM :%v", cacheUpdate)
		jsonCache, _ := json.Marshal(cache)
		w.Write(jsonCache)
	}
}

func main() {

	cache = Cache{
		FooPath:  "/foo",
		FooQuota: 10,
		FooUsed:  0,

		BarPath:  "/bar",
		BarQuota: 10,
		BarUsed:  0,

		BazPath:  "/baz",
		BazQuota: 10,
		BazUsed:  0,
	}

	fmt.Println(cache)
	http.HandleFunc("/auth", authHandler)
	http.HandleFunc("/cache", cacheHandler)

	http.ListenAndServe(":8000", nil)
}
