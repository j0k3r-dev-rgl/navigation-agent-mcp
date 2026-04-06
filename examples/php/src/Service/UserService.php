<?php

namespace App\Service;

use App\Domain\User;
use App\Repository\UserRepository;

class UserService
{
    private UserRepository $repository;

    public function __construct(UserRepository $repository)
    {
        $this->repository = $repository;
    }

    /**
     * @return User[]
     */
    public function listUsers(): array
    {
        $users = $this->repository->list();
        return $this->filterActiveUsers($users);
    }

    public function getUserById(string $id): ?User
    {
        $user = $this->repository->findById($id);
        if ($user === null) {
            $this->logUserNotFound($id);
            return null;
        }
        return $this->enrichUserData($user);
    }

    public function updateUser(string $id, string $name, string $email): ?User
    {
        $existing = $this->repository->findById($id);
        if ($existing === null) {
            $this->logUserNotFound($id);
            return null;
        }

        $input = $this->buildUpdateUserInput($name, $email);
        $updated = $this->createDomainUser($input);
        return $this->persistUser($updated);
    }

    public function createUser(string $name, string $email): User
    {
        $input = $this->buildCreateUserInput($name, $email);
        $user = $this->createDomainUser($input);
        return $this->persistUser($user);
    }

    private function buildCreateUserInput(string $name, string $email): array
    {
        $input = $this->normalizeCreateUserInput($name, $email);
        $this->validateCreateUserInput($input);
        return $input;
    }

    private function normalizeCreateUserInput(string $name, string $email): array
    {
        return [
            'name' => $this->normalizeName($name),
            'email' => $this->normalizeEmail($email),
        ];
    }

    private function validateCreateUserInput(array $input): void
    {
        $this->requireValue('name', $input['name']);
        $this->requireValue('email', $input['email']);
    }

    private function requireValue(string $field, string $value): void
    {
        if (empty($value)) {
            throw new \InvalidArgumentException("$field is required");
        }
    }

    private function createDomainUser(array $input): User
    {
        return new User(
            $this->buildUserID($input['email']),
            $input['name'],
            $input['email']
        );
    }

    private function buildUserID(string $email): string
    {
        return $this->generateUserID($email);
    }

    private function persistUser(User $user): User
    {
        return $this->repository->save($user);
    }

    private function normalizeName(string $name): string
    {
        return trim($name);
    }

    private function normalizeEmail(string $email): string
    {
        return trim(strtolower($email));
    }

    private function generateUserID(string $email): string
    {
        $parts = explode('@', $email);
        if (empty($parts[0])) {
            return 'user-generated';
        }

        return 'user-' . $parts[0];
    }

    private function buildUpdateUserInput(string $name, string $email): array
    {
        $input = $this->normalizeCreateUserInput($name, $email);
        $this->validateCreateUserInput($input);
        return $input;
    }

    private function filterActiveUsers(array $users): array
    {
        return array_filter($users, function ($user) {
            return $this->isActiveUser($user);
        });
    }

    private function isActiveUser(User $user): bool
    {
        return !empty($user->getName()) && !empty($user->getEmail());
    }

    private function enrichUserData(User $user): User
    {
        // Simula enriquecimiento de datos
        return $user;
    }

    private function logUserNotFound(string $id): void
    {
        error_log("User not found: {$id}");
    }
}
