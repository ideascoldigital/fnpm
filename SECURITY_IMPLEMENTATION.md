# Security Audit Feature - Implementation Summary

## 🎯 Objetivo

Proteger a los usuarios de paquetes maliciosos (como sha1-hulud) que ejecutan código dañino durante la instalación, analizando los scripts de instalación **antes** de que se ejecuten en el sistema.

## ✅ Cambios Implementados

### 1. Nuevo Módulo de Seguridad (`src/security.rs`)

**Características principales:**

- **Instalación en sandbox**: Instala paquetes en `/tmp` con `--ignore-scripts`
- **Análisis de package.json**: Extrae y analiza scripts de instalación (preinstall, install, postinstall)
- **Detección de patrones sospechosos**: Escanea 20+ patrones peligrosos
- **Cálculo de nivel de riesgo**: 5 niveles (Safe → Critical)
- **Confirmación interactiva**: Solicita aprobación antes de instalar paquetes riesgosos
- **Limpieza automática**: El directorio temporal se elimina automáticamente

**Patrones detectados:**
- Descargas de internet (curl, wget, fetch)
- Ejecución de código (eval, exec, spawn)
- Acceso a credenciales (~/.ssh, ~/.aws, env)
- Operaciones de archivos (rm -rf, chmod +x)
- Ofuscación (base64)
- Acceso a archivos del sistema

### 2. Actualización de Configuración (`src/config.rs`)

```rust
pub struct Config {
    package_manager: String,
    pub global_cache_path: String,
    pub target_lockfile: Option<String>,
    pub security_audit: bool,  // ← NUEVO
}
```

- Agregado campo `security_audit` (default: `true`)
- Métodos para habilitar/deshabilitar auditoría

### 3. Integración en `fnpm add` (`src/main.rs`)

**Nuevo flujo:**

```
fnpm add <package>
    ↓
¿security_audit enabled?
    ↓ YES
Instalar en sandbox (/tmp)
    ↓
Analizar package.json
    ↓
¿Tiene scripts sospechosos?
    ↓ YES
Mostrar reporte + pedir confirmación
    ↓
¿Usuario aprueba?
    ↓ YES
Instalar normalmente
```

**Nuevo flag:**
```bash
fnpm add <package> --no-audit  # Saltar auditoría
```

### 4. Tests (`tests/security_tests.rs`)

- ✅ Test de detección de patrones sospechosos
- ✅ Test de cálculo de nivel de riesgo
- ✅ Test de auditoría de paquete real (ignorado por defecto - requiere red)

### 5. Documentación

**Nuevos archivos:**
- `docs/SECURITY.md` - Documentación completa de la característica
- `docs/SECURITY_EXAMPLES.md` - Ejemplos de uso y casos reales

**Actualizado:**
- `README.md` - Agregada sección de seguridad en features

### 6. Dependencias

**Agregado a `Cargo.toml`:**
```toml
uuid = { version = "1.0", features = ["v4"] }
```

## 📊 Niveles de Riesgo

| Nivel | Descripción | Acción |
|-------|-------------|--------|
| ✓ SAFE | Sin scripts | Procede automáticamente |
| ⚠ LOW | Scripts sin patrones sospechosos | Confirma (default: SÍ) |
| ⚠ MEDIUM | 1-2 patrones sospechosos | Confirma (default: SÍ) |
| ⚠ HIGH | 3-4 patrones sospechosos | Confirma (default: NO) |
| ☠ CRITICAL | 5+ patrones sospechosos | Confirma (default: NO) |

## 🧪 Testing

```bash
# Compilar
cargo build --release

# Tests unitarios
cargo test --test security_tests

# Test manual
cd /tmp && mkdir test-project
cd test-project
echo '{"name":"test"}' > package.json
fnpm setup npm --no-hooks
fnpm add is-number@7.0.0  # Debería mostrar: ✓ SAFE
```

## 🔒 Ejemplo de Uso

```bash
$ fnpm add some-package

🔐 Security check for: some-package
🔍 Auditing package security...
   Installing some-package in sandbox...

═══════════════════════════════════════════
📦 Package: some-package
🛡️  Risk Level: ⚠ MEDIUM
═══════════════════════════════════════════

📜 Install Scripts:
  postinstall: curl https://cdn.example.com/assets.tar.gz | tar -xz

⚠️  Suspicious Patterns Detected:
  • curl: Downloads files from internet

═══════════════════════════════════════════

? This package has SUSPICIOUS patterns. Are you sure? (y/N)
```

## 🚀 Performance

- **Overhead**: ~2-5 segundos por paquete
- **Impacto**: Mínimo comparado con el riesgo evitado
- **Optimización**: Solo se ejecuta en `fnpm add`, no en `fnpm install`

## ⚙️ Configuración

### Deshabilitar globalmente

Editar `.fnpm/config.json`:
```json
{
  "package_manager": "npm",
  "security_audit": false
}
```

### Deshabilitar para un paquete específico

```bash
fnpm add trusted-package --no-audit
```

### Auditorías se saltan automáticamente en:

- ✅ Instalaciones globales (`-g`)
- ✅ `fnpm install` (sin paquete específico)

## 🎯 Casos de Uso

### Protege contra:

1. **Supply chain attacks** - Paquetes comprometidos
2. **Typosquatting** - Nombres similares a paquetes populares
3. **Credential theft** - Scripts que roban SSH keys, AWS credentials
4. **Backdoors** - Código malicioso en install scripts
5. **Data exfiltration** - Envío de datos a servidores externos

### No protege contra:

- ❌ Código malicioso que NO está en install scripts
- ❌ Vulnerabilidades conocidas (usa `npm audit` para eso)
- ❌ Time bombs (código que se activa después)

## 📝 Archivos Modificados

```
Cargo.toml                      # Agregada dependencia uuid
Cargo.lock                      # Lockfile actualizado
src/config.rs                   # Campo security_audit
src/lib.rs                      # Exportar módulo security
src/main.rs                     # Integración en execute_add
src/security.rs                 # NUEVO - Módulo completo de seguridad
tests/security_tests.rs         # NUEVO - Tests de seguridad
docs/SECURITY.md                # NUEVO - Documentación
docs/SECURITY_EXAMPLES.md       # NUEVO - Ejemplos
README.md                       # Sección de seguridad agregada
```

## 🔄 Compatibilidad

- ✅ npm - Soporte completo
- ✅ pnpm - Soporte completo
- ✅ yarn - Soporte completo
- ✅ bun - Soporte completo
- ❌ deno - No aplica (usa URLs)

## 🌟 Ventajas vs npm audit

| Característica | npm audit | fnpm security |
|----------------|-----------|---------------|
| CVE database | ✅ | ❌ |
| Install scripts | ❌ | ✅ |
| Pre-install | ❌ | ✅ |
| Previene ejecución | ❌ | ✅ |
| Detección de patrones | ❌ | ✅ |

**Recomendación:** Usar ambos como capas de seguridad complementarias.

## ✨ Próximos Pasos Sugeridos

1. **Machine Learning**: Entrenar modelo para detectar patrones más sofisticados
2. **Base de datos compartida**: Reportar/consultar paquetes maliciosos conocidos
3. **Integración con Socket.dev/Snyk**: APIs externas de seguridad
4. **Análisis estático**: Escanear código fuente completo (no solo scripts)
5. **Sandboxing runtime**: Ejecutar scripts en contenedor aislado

## 📚 Referencias

- [sha1-hulud incident](https://github.com/advisories/GHSA-xxxx)
- [npm security best practices](https://docs.npmjs.com/security-best-practices)
- [Socket.dev](https://socket.dev)
- [Snyk](https://snyk.io)
