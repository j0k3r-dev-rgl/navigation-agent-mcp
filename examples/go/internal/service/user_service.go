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

type createUserInput struct {
	name  string
	email string
}

func NewUserService(repository repository.UserRepository) *UserService {
	return &UserService{repository: repository}
}

func (s *UserService) ListUsers() []domain.User {
	return s.repository.List()
}

func (s *UserService) CreateUser(name string, email string) (domain.User, error) {
	input, err := buildCreateUserInput(name, email)
	if err != nil {
		return domain.User{}, err
	}

	user := createDomainUser(input)
	return s.persistUser(user), nil
}

func buildCreateUserInput(name string, email string) (createUserInput, error) {
	input := normalizeCreateUserInput(name, email)
	if err := validateCreateUserInput(input); err != nil {
		return createUserInput{}, err
	}
	return input, nil
}

func normalizeCreateUserInput(name string, email string) createUserInput {
	return createUserInput{
		name:  normalizeName(name),
		email: normalizeEmail(email),
	}
}

func validateCreateUserInput(input createUserInput) error {
	if err := requireValue("name", input.name); err != nil {
		return err
	}
	return requireValue("email", input.email)
}

func requireValue(field string, value string) error {
	if value == "" {
		return fmt.Errorf("%s is required", field)
	}
	return nil
}

func createDomainUser(input createUserInput) domain.User {
	return domain.NewUser(buildUserID(input.email), input.name, input.email)
}

func buildUserID(email string) string {
	return generateUserID(email)
}

func (s *UserService) persistUser(user domain.User) domain.User {
	return s.repository.Save(user)
}

func normalizeName(name string) string {
	return strings.TrimSpace(name)
}

func normalizeEmail(email string) string {
	return strings.TrimSpace(strings.ToLower(email))
}

func generateUserID(email string) string {
	parts := strings.Split(email, "@")
	if len(parts) == 0 || parts[0] == "" {
		return "user-generated"
	}

	return "user-" + parts[0]
}
