# Publicación de releases

## Objetivo

Dejar un flujo simple y correcto para publicar nuevas versiones en GitHub Releases y npm sin desalinear:

- `package.json`
- `package-lock.json`
- `packages/*/package.json`
- `crates/navigation-engine/Cargo.toml`
- `.release-please-manifest.json`
- tags y releases de GitHub
- versiones publicadas en npm

---

## Regla principal

La fuente de verdad operativa para la próxima release es:

1. lo que ya está publicado en npm
2. `.release-please-manifest.json`
3. la PR automática de Release Please

No crear tags manuales si no es estrictamente necesario.

---

## Estado esperado de la configuración

### Workflow

- `Release Please` corre en cada push a `main`
- `Release` se dispara por tags `v*`

### Release Please

El repo debe mantener:

- `.release-please-manifest.json` alineado con la última versión real publicada
- `release-please-config.json` con:
  - `include-component-in-tag: false`
  - `extra-files` apuntando a todos los archivos de versión necesarios

### npm

- `NPM_TOKEN` configurado en GitHub Actions

---

## Flujo correcto para una nueva release

### 1. Confirmar la última versión publicada real

Antes de hacer nada, revisar npm y GitHub:

```bash
npm view @navigation-agent/mcp-server version
gh release list --limit 5
```

La versión base debe ser la misma en ambos lados.

Si npm dice `0.3.4`, entonces:

- `.release-please-manifest.json` debe estar en `0.3.4`
- la próxima release debe partir desde `0.3.4`

---

### 2. Hacer push de los cambios del producto a `main`

Los commits siguen conventional commits.

```bash
git push origin main
```

Importante:

- `fix:` genera patch
- `feat:` genera minor

Si quieres forzar una versión exacta, usar `release-as` temporalmente en `release-please-config.json`.

Ejemplo:

```json
"release-as": "0.3.5"
```

Eso solo se usa para corregir o fijar una versión puntual. Después de publicar, se elimina.

---

### 3. Esperar la PR automática de Release Please

```bash
gh pr list --state open --limit 10
```

La PR correcta debe:

- tener la versión esperada
- actualizar `CHANGELOG.md`
- actualizar todas las versiones alineadas

Revisar especialmente que cambie:

- `package.json`
- `package-lock.json`
- `packages/mcp-server/package.json`
- `packages/contract-tests/package.json`
- `packages/mcp-server-*/package.json`
- `crates/navigation-engine/Cargo.toml`
- `.release-please-manifest.json`

---

### 4. Verificar que Release Please no inventó una versión incorrecta

Si la PR propone una versión inesperada:

1. revisar `.release-please-manifest.json`
2. revisar si hubo commits `feat:` que estén empujando minor
3. si hace falta, fijar temporalmente `release-as`

Si el changelog arrastra commits viejos, se puede usar temporalmente:

```json
"last-release-sha": "<sha-del-último-release-real>"
```

Después de corregir la PR y publicar, remover ese override.

---

### 5. Mergear la PR de release

```bash
gh pr merge <numero> --squash --admin
```

No crear el tag manualmente salvo que el workflow no lo haga.

Con la configuración actual correcta, Release Please debe crear el tag en formato:

```bash
vX.Y.Z
```

No `navigation-agent-mcp-vX.Y.Z`.

---

### 6. Verificar el workflow de publicación

```bash
gh run list --workflow=release.yml --limit=5
gh run watch <run-id> --exit-status
```

Si termina bien, deben publicarse:

- release de GitHub
- paquetes binarios por plataforma
- `@navigation-agent/mcp-server`

---

### 7. Verificar publicación real

GitHub:

```bash
gh release list --limit 5
```

npm:

```bash
npm view @navigation-agent/mcp-server version
npm view @navigation-agent/mcp-server-linux-x64 version
```

La versión debe coincidir en:

- GitHub Release
- npm principal
- paquetes binarios

---

## Qué no hacer

- no crear tags manuales por costumbre
- no empujar una versión si npm todavía no refleja la anterior
- no dejar `release-as` permanente
- no dejar `last-release-sha` permanente
- no asumir que `package.json` por sí solo controla Release Please

---

## Qué hacer si algo queda desalineado

Caso típico:

- GitHub quedó en `0.3.4`
- npm sigue en `0.3.3`

En ese caso:

1. no seguir publicando
2. cerrar la PR automática siguiente
3. borrar release/tag incorrectos de GitHub si npm no publicó esa versión
4. volver el repo a la última versión real publicada
5. corregir `.release-please-manifest.json`
6. recién ahí preparar la próxima release

---

## Error común: `npm install` / `npm ci` con `Invalid Version:`

Revisar:

1. `packages/mcp-server/package.json`
   - `version`
   - `optionalDependencies`

2. `packages/mcp-server-*/package.json`
   - todos con la misma versión

3. `packages/contract-tests/package.json`
   - con `version` válida

4. `package-lock.json`
   - regenerado y alineado

5. `Cargo.toml` / `Cargo.lock`
   - alineados con la versión del repo si se decidió versionar Rust junto al producto

Validación local:

```bash
rm package-lock.json
npm install --package-lock-only
npm ci
```

---

## Configuración recomendada de Release Please

Puntos importantes:

- `include-component-in-tag: false`
  - para que los tags sean `vX.Y.Z`

- `extra-files`
  - debe incluir todos los archivos de versión del repo

- `release-as`
  - solo temporal cuando quieras forzar una versión exacta

- `last-release-sha`
  - solo temporal cuando el changelog quedó corrido

---

## Flujo corto recomendado

Si todo está sano, la secuencia normal es:

```bash
git push origin main
gh pr list --state open --limit 10
gh pr merge <release-pr> --squash --admin
gh run list --workflow=release.yml --limit=5
gh run watch <run-id> --exit-status
npm view @navigation-agent/mcp-server version
gh release list --limit 5
```

---

## Convención práctica

- si la release anterior no quedó publicada en npm, no seguir con la siguiente
- primero resincronizar GitHub con npm
- luego preparar la siguiente release
- si hace falta fijar una versión exacta, usar `release-as` temporalmente
- después de publicar, quitar los overrides temporales
