# Publicación de releases

## Objetivo

Documentar el flujo correcto para publicar una nueva versión en GitHub Releases y npm.

---

## Prerrequisitos

### GitHub

- Workflow `Release Please` configurado
- Workflow `Release` configurado
- Secret `PAT` configurado en GitHub Actions para que `release-please` pueda crear PRs

### npm

- Secret `NPM_TOKEN` configurado en GitHub Actions

### Repo

- Todos los `package.json` deben tener versiones consistentes
- `package-lock.json` debe estar actualizado
- `packages/mcp-server/package.json` debe tener `optionalDependencies` alineadas con la versión release actual

---

## Flujo normal de publicación

### 1. Subir cambios a `main`

Los commits deben seguir conventional commits (`feat:`, `fix:`, `docs:`, etc.).

```bash
git push origin main
```

---

### 2. Esperar el PR automático de Release Please

El workflow `Release Please` corre en cada push a `main` y crea un PR de release.

Ese PR normalmente:

- actualiza `CHANGELOG.md`
- actualiza versiones
- prepara la nueva release

---

### 3. Mergear el PR de release

Se puede hacer desde GitHub o con `gh`:

```bash
gh pr list
gh pr merge <numero> --squash --admin
```

---

### 4. Verificar que exista el tag

El workflow `Release` se dispara por tags `v*`.

Si el tag no se creó automáticamente, crearlo manualmente:

```bash
git pull origin main
git tag vX.Y.Z
git push origin vX.Y.Z
```

Ejemplo:

```bash
git tag v0.3.0
git push origin v0.3.0
```

---

### 5. Verificar el workflow de publicación

```bash
gh run list --workflow=release.yml --limit=5
gh run watch <run-id> --exit-status
```

Si termina bien, se publican:

- los paquetes binarios por plataforma
- el paquete principal `@navigation-agent/mcp-server`

---

### 6. Verificar la publicación

GitHub Releases:

```bash
gh release list
```

npm:

- revisar `https://www.npmjs.com/package/@navigation-agent/mcp-server`

---

## Qué revisar si falla la publicación

### Error en `npm ci` con `Invalid Version:`

Revisar:

1. `packages/mcp-server/package.json`
   - `optionalDependencies` deben apuntar a la misma versión release

2. `packages/mcp-server-*/package.json`
   - todos los paquetes de plataforma deben tener la misma versión

3. `packages/contract-tests/package.json`
   - debe incluir campo `version`

4. `package-lock.json`
   - regenerarlo si quedó desalineado

Validación local:

```bash
npm install --package-lock-only
npm ci
```

---

## Reintentar una release fallida sin cambiar versión

Si el problema es de publicación o pipeline y **no del producto**, reutilizar el mismo tag.

Ejemplo con `v0.3.0`:

```bash
git push origin main
git push origin --delete v0.3.0
git tag -d v0.3.0
git tag v0.3.0
git push origin v0.3.0
```

Esto vuelve a disparar el workflow `Release` para la misma versión.

---

## Convención recomendada

- Si falla el pipeline de publicación, **no subir versión nueva** si el producto no cambió
- Corregir el pipeline
- Reusar el mismo tag
- Solo crear nueva versión si cambió el código del producto o el contenido release esperado
