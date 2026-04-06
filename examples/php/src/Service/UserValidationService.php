<?php

namespace App\Service;

use App\Repository\UserRepository;

class UserValidationService
{
    private UserRepository $repository;

    public function __construct(UserRepository $repository)
    {
        $this->repository = $repository;
    }

    public function isEmailAvailable(string $email): bool
    {
        $users = $this->repository->list();
        return $this->checkEmailUniqueness($users, $email);
    }

    public function userExists(string $id): bool
    {
        $user = $this->repository->findById($id);
        return $user !== null;
    }

    private function checkEmailUniqueness(array $users, string $email): bool
    {
        foreach ($users as $user) {
            if ($this->normalizeEmail($user->getEmail()) === $this->normalizeEmail($email)) {
                return false;
            }
        }
        return true;
    }

    private function normalizeEmail(string $email): string
    {
        return trim(strtolower($email));
    }
}
