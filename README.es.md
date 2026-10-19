<div align="center">

**[English](README.md)** | **[Español](README.es.md)** | **[中文](README.zh-CN.md)** | **[日本語](README.ja.md)** | **[Français](README.fr.md)**

</div>

<div align="center">
  <img src="branding/logo.png" width="220" alt="Sanctifier" />

  # Sanctifier

  ### Atrapa el error antes de que otro lo aproveche.

  **Copiloto de seguridad para contratos inteligentes Stellar Soroban** — análisis estático, verificación formal con Z3, guardias de ejecución en cadena y un panel amigable para auditores, todo impulsado por un único motor limpio SARIF.

  [![CI](https://github.com/HyperSafeD/Sanctifier/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/HyperSafeD/Sanctifier/actions/workflows/ci.yml)
  [![Codecov](https://codecov.io/gh/HyperSafeD/Sanctifier/graph/badge.svg)](https://codecov.io/gh/HyperSafeD/Sanctifier)
  [![crates.io](https://img.shields.io/crates/v/sanctifier-cli.svg)](https://crates.io/crates/sanctifier-cli)
  [![Soroban Testnet](https://img.shields.io/badge/Soroban%20Testnet-Live-2dd4bf?style=flat-square&logo=stellar)](LIVE_TESTNET.md)
  [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
</div>

---

## Por qué existe Sanctifier

Cuando un contrato EVM lanza un error, la comunidad tiene una década de herramientas — Slither, Mythril, Foundry, Certora — para atraparlo. Soroban se lanzó a mainnet en 2024 con casi ninguno de esos andamios. Cada equipo escribe la misma lista de verificación desde cero. Cada auditoría redescubre los mismos cinco errores peligrosos.

Sanctifier es la capa que faltaba. **Un motor, doce reglas canónicas, tres superficies de despliegue.** Construido específicamente para el modelo de autorización de Soroban, semántica TTL de almacenamiento, interfaz de token SEP-41 y peculiaridades de gas/eventos. Código abierto. Nivel auditor. Listo para CI.

---

## Lo que detecta

Cada hallazgo tiene un código estable — `S001..S012` — para que puedas filtrar, suprimir y hacer seguimiento a través de versiones.

| Código | Lo que detecta | Por qué es peligroso |
|------|-----------------|--------------|
| `S001` | Falta `require_auth` en llamadas que cambian estado | Cualquiera puede drenar tu contrato |
| `S002` | `panic!` / `unwrap` / `expect` en rutas del contrato | Estado bloqueado, sin recuperación |
| `S003` | Aritmética no verificada — desbordamiento, subdesbordamiento, truncamiento | Pérdida silenciosa de fondos por redondeo |
| `S004` | Entradas de ledger empujando el umbral de tamaño | Rechazo en tiempo de escritura, a mitad de transacción |
| `S005` | Colisiones de claves de almacenamiento entre rutas de datos | Corrupción de datos entre características |
| `S006` | Patrones inseguros — incluyendo timestamp como aleatoriedad | Ganadores predecibles, explotación de repetición |
| `S007` | Tus reglas YAML personalizadas | Tu estilo de casa, forzado |
| `S008` | Emisiones de eventos inconsistentes o faltantes | Billeteras e indexadores se quedan a ciegas |
| `S009` | Valores de retorno `Result` no manejados | Fallos silenciosos disfrazados de éxito |
| `S010` | Riesgo de actualización / admin / gobernanza | Rutas de toma de control con una sola clave |
| `S011` | Invariantes refutados por Z3 | Garantías matemáticas que no tienes |
| `S012` | Desviaciones de interfaz de token SEP-41 | Billeteras rechazan tu token |

Además la **base de datos de vulnerabilidades** de la comunidad coincide patrones estilo CVE conocidos (`SOL-2024-*`) contra tu AST — así que una explotación publicada en cualquier lugar se convierte en un hallazgo en todas partes.

---

## En vivo en Soroban testnet — ahora mismo

Esto no es una presentación. El **Wrapper de Guardia de Ejecución**, **Guardia de Reentrada**, y **Contrato Vulnerable por Diseño** de Sanctifier están desplegados y emitiendo eventos de auditoría en cadena contra los que puedes ejecutar `stellar contract invoke` hoy. Consulta **[LIVE_TESTNET.md](LIVE_TESTNET.md)** para direcciones, comandos de verificación y registros de eventos.

```bash
# Seguir eventos de guardia en tiempo real en el despliegue en vivo
stellar events --network testnet --start-ledger <LATEST> \
  --id $RUNTIME_GUARD_CONTRACT_ID
```

---

## Cinco formas de usarlo

| Superficie | Para | Tiempo al primer hallazgo |
|---|---|---|
| **CLI `sanctifier`** | Desarrollo local, scripts, rutas calientes | **30 segundos** |
| **Acción de GitHub** | Cada PR, cada push | **Un commit** |
| **Panel Web** (Next.js) | Auditores, revisores, demos de hackathon | Arrastra y suelta un archivo `.rs` |
| **Extensión VS Code** | Diagnósticos en línea mientras escribes | Una instalación |
| **Guardia de Ejecución en cadena** | Ruta forense después del despliegue | Un envoltorio de contrato |

El mismo motor debajo de todos (se compila cruzado a WASM para la ruta del navegador), así que los hallazgos son idénticos bit a bit dondequiera que escanees.

---

## Inicio rápido de 30 segundos

```bash
# 1. instalar
cargo install sanctifier-cli

# 2. escanear
sanctifier analyze ./contracts/my-token

# 3. enviar una insignia para tu README
sanctifier analyze . --format json > report.json
sanctifier badge --report report.json --svg-output sanctifier.svg
```

<details>
<summary><b>Lo que verás</b></summary>

```text
⚠️ Gaps de Autenticación
   → [S001] src/lib.rs:transfer — falta require_auth
   → [S001] src/lib.rs:mint     — falta require_auth

⚠️ Aritmética No Verificada
   → [S003] src/lib.rs:transfer:30 — operador `-`
   → [S003] src/lib.rs:transfer:33 — operador `+`

⚠️ Desviación SEP-41
   → [S012] falta función `allowance`

🛡️ 2 coincidencias de vulnerabilidades conocidas de DB v1.0.0
   ❌ [SOL-2024-002] Falta auth en transferencia de token (CRÍTICO)
   🔴 [SOL-2024-003] Subdesbordamiento de saldo no verificado (ALTO)

✨ Escaneo completo · 4 hallazgos · exit 1
```

El código de salida es `1` cuando hay hallazgos críticos/alto — conéctalo a CI tal cual.

</details>

---

## Conéctalo a tu repo (en un PR)

```yaml
# .github/workflows/sanctifier.yml
name: Sanctifier
on: [pull_request, push]
permissions: { contents: read, security-events: write }
jobs:
  scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: HyperSafeD/Sanctifier@main
        with:
          path: .
          format: sarif
          min-severity: high
          upload-sarif: "true"
```

SARIF aterriza en GitHub code-scanning para que los revisores vean anotaciones en línea en PRs.

---

## Ejecuta el panel localmente

```bash
cd frontend
npm install
npm run dev
# → http://localhost:3000
```

- **`/scan`** — arrastra un archivo `.rs`, obtén hallazgos en <2s
- **`/dashboard`** — carga un reporte JSON, profundiza por severidad, ve un grafo de llamadas en vivo
- **`/playground`** — prueba contratos vulnerables enlatados (gap de auth, desbordamiento, PRNG inseguro, …)
- **`/terminal`** — `sanctifier` en un emulador de terminal para demos guiadas

---

## Opciones de instalación

| Método | Comando |
|--------|---------|
| **crates.io** | `cargo install sanctifier-cli` |
| **Desde fuente** | `git clone https://github.com/HyperSafeD/Sanctifier && cd Sanctifier && make release` |
| **Codespaces** | [![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/HyperSafeD/Sanctifier) |
| **Docker** | `docker run --rm -v $PWD:/src ghcr.io/hypersafed/sanctifier analyze /src` |

**Requisitos previos:** Rust 1.78+, además de `libz3-dev` y `clang`/`libclang-dev` para el backend de verificación formal Z3.

```bash
# Debian/Ubuntu
sudo apt-get install libz3-dev clang libclang-dev
# macOS
brew install z3 llvm
```

Omite Z3 completamente con `cargo install sanctifier-cli --no-default-features` — cada regla excepto `S011` aún se ejecuta.

---

## Referencia de CLI

```bash
sanctifier analyze    [PATH] [--format text|json] [--limit BYTES] [--webhook-url URL]...
sanctifier diff       [PATH] --baseline <report.json>
sanctifier watch      [PATH]              # se reejecuta al cambiar archivo
sanctifier workspace  [PATH]              # escaneo consciente de cargo-workspace
sanctifier callgraph  [PATH] --output callgraph.dot
sanctifier badge      --report report.json --svg-output sanctifier.svg
sanctifier fix        [PATH] --rule S003  # aplicar parches del fixer
sanctifier verify     [PATH]              # pase de invariantes solo Z3
sanctifier deploy     ...                 # enviar la guardia de ejecución
sanctifier doctor                         # diagnósticos de entorno
sanctifier init                           # generar .sanctify.toml
sanctifier update                         # autoactualización con verificación de checksum
```

Cada subcomando respeta `--format json` para consumo por máquina.

---

## La salida es un contrato, no una vibra

La salida `--format json` valida contra [`schemas/analysis-output.json`](schemas/analysis-output.json) (JSON Schema draft-07). Cada reporte lleva un `schema_version` que aumenta independientemente de la versión CLI, así que las herramientas descendentes pueden fijarse a un esquema sin acoplarse a un ritmo de lanzamiento.

```jsonc
{
  "metadata":       { "version": "0.1.0", "format": "sanctifier-ci-v1", "timestamp": "…" },
  "summary":        { "critical": 0, "high": 0, "medium": 2, "low": 0 },
  "findings":       { "auth_gaps": [...], "arithmetic_issues": [...], "storage_collisions": [...] },
  "vuln_db_matches": [{ "id": "SOL-2024-002", "severity": "CRITICAL", "matched_at": "…" }],
  "schema_version": "1.0.0"
}
```

La salida SARIF 2.1.0 es canónica para GitHub code-scanning y cualquier agregador SAST.

---

## Configuración — `.sanctify.toml`

```toml
ignore_paths        = ["target", ".git"]
enabled_rules       = ["auth_gaps", "panics", "arithmetic", "ledger_size"]
ledger_limit        = 64000
approaching_threshold = 0.8
strict_mode         = false

[[custom_rules]]
name     = "no_unsafe_block"
pattern  = 'unsafe\s*\{'
severity = "error"
```

Las reglas personalizadas soportan DSL YAML completo — consulta [docs/rule-authoring-guide.md](docs/rule-authoring-guide.md).

---

## Hoja de ruta

Sanctifier se envía en oleadas. Lo que está hecho, lo que sigue, lo que es deseado:

**Enviado**
- 12 reglas de análisis canónicas (S001–S012) con códigos estables
- CLI, Acción de GitHub, Panel Web, Extensión VS Code, compilación WASM
- Contratos de guardia de ejecución testnet en vivo emitiendo eventos de auditoría en cadena
- Salida SARIF + JSON, esquema draft-07, generador de insignias
- Modo diff, modo watch, escaneo cargo-workspace, parcheador

**En curso** (consulta los [issues contrib-wave](https://github.com/HyperSafeD/Sanctifier/issues?q=contrib-wave+in%3Atitle))
- Proveedor LLM real para `/api/ai/explain` (actualmente stub)
- `sanctifier lsp` agnóstico de editor para Neovim / Helix / Zed
- Salida `--ndjson` streaming para tubería incremental
- Formateador de comentarios de PR de GitHub con delta vs base
- 20+ nuevas reglas del motor (carrera de allowance, bumps TTL, `try_call` entre contratos, taint a través de destructuras, …)

**Lista de deseos**
- API REST alojada, plugin Stellar Laboratory, subcomando shim cargo-sanctify, motor de reglas de detección de anomalías para llamadas de ejecución registradas

---

## Diseño del proyecto

```text
Sanctifier/
├── tooling/
│   ├── sanctifier-cli/        # Binario CLI (el que instalas)
│   ├── sanctifier-core/       # Motor de análisis estático + backend Z3
│   └── sanctifier-wasm/       # Compilación WASM navegador/Node del motor
├── frontend/                  # Panel Next.js, playground, terminal
├── vscode-extension/          # Integración de diagnósticos VS Code
├── contracts/                 # Contratos Soroban (fixtures + objetivos en vivo)
│   ├── runtime-guard-wrapper/ # ← desplegado a testnet
│   ├── reentrancy-guard/      # ← desplegado a testnet
│   └── vulnerable-contract/   # ← desplegado a testnet (objetivo demo)
├── schemas/
│   └── analysis-output.json   # JSON Schema (draft-07) — validado en CI
├── data/
│   └── vulnerability-db.json  # Patrones estilo CVE de la comunidad
├── action.yml                 # Acción compuesta de GitHub
├── benchmarks/                # Corporas de rendimiento
├── specs/                     # OpenAPI + borradores RFC
└── docs/                      # Guías, ADRs, modelos de amenaza, estudios de caso
```

---

## Documentación

| Si quieres… | Lee |
|-----------------|------|
| Empezar en 10 minutos | [docs/getting-started.md](docs/getting-started.md) |
| Entender cada código de error | [docs/error-codes.md](docs/error-codes.md) |
| Conectar la guardia de ejecución a tu contrato | [docs/runtime-guards-integration.md](docs/runtime-guards-integration.md) |
| Configurar CI | [docs/ci-cd-setup.md](docs/ci-cd-setup.md) |
| Desplegar a testnet | [docs/soroban-deployment.md](docs/soroban-deployment.md) |
| Escribir tu propia regla | [docs/rule-authoring-guide.md](docs/rule-authoring-guide.md) |
| Verlo evaluado | [docs/case-studies/soroban-examples.md](docs/case-studies/soroban-examples.md) |
| Revisar el modelo de amenaza | [docs/security-threat-model.md](docs/security-threat-model.md) |
| Explorar decisiones de diseño | [docs/adr/](docs/adr/) |

---

## Contribuir

Estamos ganando impulso y queremos ayuda. **~100 issues [`[contrib-wave]`](https://github.com/HyperSafeD/Sanctifier/issues?q=contrib-wave+in%3Atitle) curados a mano** están en vivo, cada uno con declaración del problema, criterios de aceptación, punteros de archivo y pista de dificultad. Hay un `good first issue` para cada nivel de habilidad — bash, Rust, TypeScript, Next.js, GitHub Actions, escritura de docs, autoría de contratos.

Empieza con [CONTRIBUTING.md](CONTRIBUTING.md), luego elige un issue y saluda.

---

## Licencia

MIT — consulta [LICENSE](LICENSE).

<div align="center">
  <sub>Construido para el ecosistema Stellar Soroban · Mainnet no perdona · Nivel auditor, en CI.</sub>
</div>
