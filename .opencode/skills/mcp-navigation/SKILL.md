---
name: mcp-navigation
description: Usa el MCP de navegación para explorar, ubicar y analizar código antes de leer muchos archivos o proponer cambios.
---

## What I do

Uso las tools del MCP `navigation` para:

- inspeccionar estructura del proyecto
- ubicar símbolos, endpoints y rutas
- trazar referencias y callers
- entender features con el mínimo contexto posible

## When to use me

Úsame cuando el usuario pida:

- analizar código
- entender cómo funciona una parte del proyecto
- encontrar dónde se define o usa algo
- investigar impacto de un cambio
- explorar endpoints, rutas o flujo de una feature
- identificar archivos relevantes antes de editar

## Recommended workflow

1. Primero usa el MCP `navigation` para ubicar el punto de entrada correcto.
2. Luego usa las tools más específicas posibles para reducir el área de búsqueda.
3. Lee solo los archivos mínimos necesarios.
4. Si después hace falta editar, recién ahí propone o ejecuta cambios.
5. Si el MCP no alcanza para un caso puntual, usa fallback con herramientas nativas.

## Tool selection rules

Prioriza este orden cuando aplique:

1. inspección estructural del proyecto o módulo
2. búsqueda de símbolos
3. trazado de referencias o callers
4. listado de endpoints o rutas
5. lectura puntual de archivos ya identificados

## Rules

- No hagas exploración amplia con bash si el MCP `navigation` resuelve mejor el problema.
- No leas muchos archivos sin antes acotar con el MCP.
- No adivines relaciones entre archivos si puedes trazarlas con el MCP.
- Usa siempre la tool más barata y específica posible.
- Si el usuario pide solo análisis, no edites nada.

## Fallback

Si el MCP `navigation` no resuelve completamente el caso:

- usa búsqueda nativa de contenido o símbolos
- lee archivos puntuales
- evita exploración indiscriminada
- explica brevemente que hiciste fallback

## Success criteria

Esta skill está bien usada cuando:

- se redujo la cantidad de archivos abiertos
- se identificó rápido el punto de entrada
- el análisis se hizo con menos ruido
- las conclusiones se basan en evidencia del código
