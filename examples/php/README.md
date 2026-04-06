# PHP example app

App pequeña pensada para probar herramientas de navegación.

## Estructura

```
src/
├── Domain/
│   └── User.php                      # Entidad de dominio
├── Repository/
│   ├── UserRepository.php            # Interfaz del repositorio
│   └── MemoryUserRepository.php      # Implementación en memoria
└── Service/
    ├── UserService.php               # Servicio principal (CRUD)
    ├── UserValidationService.php     # Validaciones de negocio
    └── UserExportService.php         # Exportación de datos
```

## Casos de prueba implementados

### 1. `code.find_symbol`
Detecta todos los símbolos PHP:
- **Clases**: `UserService`, `UserRepository`, `MemoryUserRepository`, etc.
- **Interfaces**: `UserRepository`
- **Métodos**: `createUser`, `updateUser`, `listUsers`, `save`, `findById`, etc.
- **Funciones**: (en este ejemplo no hay funciones globales, solo métodos)

### 2. `code.trace_flow` (rastrear llamadas salientes)

**Ejemplo 1: `UserService::createUser`**
```
createUser → buildCreateUserInput → normalizeCreateUserInput → normalizeName
                                                             → normalizeEmail
          → createDomainUser → buildUserID → generateUserID
          → persistUser → repository.save
```

**Ejemplo 2: `UserService::updateUser`**
```
updateUser → repository.findById
          → logUserNotFound
          → buildUpdateUserInput → normalizeCreateUserInput
          → createDomainUser
          → persistUser → repository.save
```

**Ejemplo 3: `UserExportService::exportToJson`**
```
exportToJson → repository.list
            → transformUsersToArray → userToArray → user.toArray
            → encodeToJson
```

### 3. `code.trace_callers` (rastrear impacto de cambios)

**¿Quién llama a `UserRepository::list`?**
- `UserService::listUsers` (línea 22)
- `UserValidationService::isEmailAvailable` (línea 18)
- `UserExportService::exportToJson` (línea 18)
- `UserExportService::exportToCsv` (línea 25)
- `UserExportService::getUserCount` (línea 32)

**¿Quién llama a `UserRepository::findById`?**
- `UserService::getUserById` (línea 28)
- `UserService::updateUser` (línea 38)
- `UserValidationService::userExists` (línea 24)

**¿Quién llama a `UserRepository::save`?**
- `UserService::persistUser` (línea 100)

## Ejecutar demo completo

Desde la raíz del proyecto:

```bash
./demo-php-navigation.sh
```

Este script demuestra todas las capacidades de navegación PHP:
- Búsqueda de símbolos en múltiples archivos
- Rastreo de flujo de ejecución con múltiples niveles de profundidad
- Análisis de impacto mostrando todos los callers de métodos críticos

## Propósito

Este ejemplo está diseñado para:
1. **Validar** que el analizador PHP detecta correctamente todos los tipos de símbolos
2. **Probar** trace_flow con flujos complejos y múltiples niveles de llamadas
3. **Demostrar** trace_callers con métodos llamados desde múltiples ubicaciones
4. **Ser framework-agnostic**: No depende de Laravel, Symfony ni ningún framework específico
