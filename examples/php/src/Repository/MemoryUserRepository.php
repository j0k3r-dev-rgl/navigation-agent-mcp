<?php

namespace App\Repository;

use App\Domain\User;

class MemoryUserRepository implements UserRepository
{
    /** @var User[] */
    private array $users = [];

    public function list(): array
    {
        return array_values($this->users);
    }

    public function save(User $user): User
    {
        $this->users[$user->getId()] = $user;
        return $user;
    }

    public function findById(string $id): ?User
    {
        return $this->users[$id] ?? null;
    }
}
