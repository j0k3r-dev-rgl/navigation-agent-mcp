<?php

namespace App\Repository;

use App\Domain\User;

interface UserRepository
{
    /**
     * @return User[]
     */
    public function list(): array;

    public function save(User $user): User;

    public function findById(string $id): ?User;
}
