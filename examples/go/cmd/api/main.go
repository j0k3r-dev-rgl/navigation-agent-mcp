package main

import (
	"log"
	"net/http"

	"examples/goapp/internal/http/handlers"
	"examples/goapp/internal/repository"
	"examples/goapp/internal/service"
)

func main() {
	repo := repository.NewMemoryUserRepository()
	service := service.NewUserService(repo)
	handler := handlers.NewUserHandler(service)

	mux := http.NewServeMux()
	mux.HandleFunc("GET /users", handler.ListUsers)
	mux.HandleFunc("POST /users", handler.CreateUser)

	log.Fatal(http.ListenAndServe(":8081", mux))
}
