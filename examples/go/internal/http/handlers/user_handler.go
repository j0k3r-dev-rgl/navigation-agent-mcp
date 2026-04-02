package handlers

import (
	"encoding/json"
	"net/http"

	"examples/goapp/internal/service"
)

type UserHandler struct {
	service *service.UserService
}

type CreateUserRequest struct {
	Name  string `json:"name"`
	Email string `json:"email"`
}

func NewUserHandler(service *service.UserService) *UserHandler {
	return &UserHandler{service: service}
}

func (h *UserHandler) ListUsers(w http.ResponseWriter, _ *http.Request) {
	users := h.service.ListUsers()
	writeJSON(w, http.StatusOK, users)
}

func (h *UserHandler) CreateUser(w http.ResponseWriter, r *http.Request) {
	var request CreateUserRequest
	if err := json.NewDecoder(r.Body).Decode(&request); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid body"})
		return
	}

	user, err := h.service.CreateUser(request.Name, request.Email)
	if err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": err.Error()})
		return
	}

	writeJSON(w, http.StatusCreated, user)
}

func writeJSON(w http.ResponseWriter, status int, data any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(data)
}
