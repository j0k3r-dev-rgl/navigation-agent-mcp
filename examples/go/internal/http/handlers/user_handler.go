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
	request, err := decodeCreateUserRequest(r)
	if err != nil {
		writeCreateUserDecodeError(w)
		return
	}

	user, err := h.service.CreateUser(request.Name, request.Email)
	if err != nil {
		writeCreateUserValidationError(w, err)
		return
	}

	writeCreatedUser(w, user)
}

func decodeCreateUserRequest(r *http.Request) (CreateUserRequest, error) {
	var request CreateUserRequest
	if err := json.NewDecoder(r.Body).Decode(&request); err != nil {
		return CreateUserRequest{}, err
	}
	return request, nil
}

func writeCreateUserDecodeError(w http.ResponseWriter) {
	writeErrorJSON(w, http.StatusBadRequest, "invalid body")
}

func writeCreateUserValidationError(w http.ResponseWriter, err error) {
	writeErrorJSON(w, http.StatusBadRequest, err.Error())
}

func writeCreatedUser(w http.ResponseWriter, user any) {
	writeJSON(w, http.StatusCreated, user)
}

func writeErrorJSON(w http.ResponseWriter, status int, message string) {
	writeJSON(w, status, map[string]string{"error": message})
}

func writeJSON(w http.ResponseWriter, status int, data any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(data)
}
