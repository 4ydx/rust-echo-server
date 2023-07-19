package main

import (
	"fmt"
	"io"
	"net/http"
)

func main() {
	resp, err := http.Get("http://localhost:9999")
	if err != nil {
		panic(err)
	}
	defer resp.Body.Close()

	b, err := io.ReadAll(resp.Body)
	if err != nil {
		panic(err)
	}
	fmt.Printf("%s", string(b))
}
