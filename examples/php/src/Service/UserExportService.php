<?php

namespace App\Service;

use App\Repository\UserRepository;

class UserExportService
{
    private UserRepository $repository;

    public function __construct(UserRepository $repository)
    {
        $this->repository = $repository;
    }

    public function exportToJson(): string
    {
        $users = $this->repository->list();
        $data = $this->transformUsersToArray($users);
        return $this->encodeToJson($data);
    }

    public function exportToCsv(): string
    {
        $users = $this->repository->list();
        $rows = $this->transformUsersToCsvRows($users);
        return $this->buildCsvContent($rows);
    }

    public function getUserCount(): int
    {
        $users = $this->repository->list();
        return count($users);
    }

    private function transformUsersToArray(array $users): array
    {
        return array_map(function ($user) {
            return $this->userToArray($user);
        }, $users);
    }

    private function transformUsersToCsvRows(array $users): array
    {
        return array_map(function ($user) {
            return $this->userToCsvRow($user);
        }, $users);
    }

    private function userToArray($user): array
    {
        return $user->toArray();
    }

    private function userToCsvRow($user): string
    {
        $data = $user->toArray();
        return implode(',', [
            $this->escapeCsvValue($data['id']),
            $this->escapeCsvValue($data['name']),
            $this->escapeCsvValue($data['email']),
        ]);
    }

    private function escapeCsvValue(string $value): string
    {
        return '"' . str_replace('"', '""', $value) . '"';
    }

    private function encodeToJson(array $data): string
    {
        return json_encode($data, JSON_PRETTY_PRINT);
    }

    private function buildCsvContent(array $rows): string
    {
        return implode("\n", $rows);
    }
}
