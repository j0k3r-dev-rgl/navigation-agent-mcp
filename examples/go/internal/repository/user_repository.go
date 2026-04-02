package repository

import "examples/goapp/internal/domain"

type UserRepository interface {
	List() []domain.User
	Save(user domain.User) domain.User
}
