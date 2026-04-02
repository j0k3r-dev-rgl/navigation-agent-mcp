package service

import (
	"fmt"
	"strings"

	"examples/goapp/internal/domain"
	"examples/goapp/internal/repository"
)

type UserService struct {
	repository repository.UserRepository
}

func NewUserService(repository repository.UserRepository) *UserService {
	return &UserService{repository: repository}
}

func (s *UserService) ListUsers() []domain.User {
	return s.repository.List()
}

func (s *UserService) CreateUser(name string, email string) (domain.User, error) {
	normalizedName := normalizeName(name)
	normalizedEmail := strings.TrimSpace(strings.ToLower(email))

	if normalizedName == "" {
		return domain.User{}, fmt.Errorf("name is required")
	}

	if normalizedEmail == "" {
		return domain.User{}, fmt.Errorf("email is required")
	}

	user := domain.NewUser(generateUserID(normalizedEmail), normalizedName, normalizedEmail)
	return s.repository.Save(user), nil
}

func normalizeName(name string) string {
	return strings.TrimSpace(name)
}

func generateUserID(email string) string {
	parts := strings.Split(email, "@")
	if len(parts) == 0 || parts[0] == "" {
		return "user-generated"
	}

	return "user-" + parts[0]
}
