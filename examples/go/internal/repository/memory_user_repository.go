package repository

import "examples/goapp/internal/domain"

type MemoryUserRepository struct {
	users []domain.User
}

func NewMemoryUserRepository() *MemoryUserRepository {
	return &MemoryUserRepository{
		users: []domain.User{
			domain.NewUser("1", "Ada Lovelace", "ada@example.com"),
			domain.NewUser("2", "Grace Hopper", "grace@example.com"),
		},
	}
}

func (r *MemoryUserRepository) List() []domain.User {
	return r.users
}

func (r *MemoryUserRepository) Save(user domain.User) domain.User {
	r.users = append(r.users, user)
	return user
}
