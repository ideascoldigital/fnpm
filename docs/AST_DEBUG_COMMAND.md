# Comando `fnpm ast-debug`

## Descripción

El comando `ast-debug` permite inspeccionar el análisis AST (Abstract Syntax Tree) de archivos JavaScript/TypeScript para verificar cómo fnpm detecta patrones de seguridad.

## Uso

```bash
fnpm ast-debug <archivo> [--verbose]
```

### Argumentos

- `<archivo>` - Ruta al archivo JavaScript/TypeScript a analizar (requerido)
- `--verbose` o `-v` - Muestra información detallada del análisis

## Ejemplos

### Análisis básico

```bash
fnpm ast-debug node_modules/negotiator/lib/encoding.js
```

**Salida:**
```
🔍 AST Security Analysis
═══════════════════════════════════════════

📄 File: node_modules/negotiator/lib/encoding.js
📊 Size: 184 lines

🌳 AST Analysis Results:
─────────────────────────────────────────
✅ No security issues detected!

═══════════════════════════════════════════
```

### Análisis con modo verbose

```bash
fnpm ast-debug node_modules/qs/lib/parse.js --verbose
```

**Salida:**
```
🔍 AST Security Analysis
═══════════════════════════════════════════

📄 File: node_modules/qs/lib/parse.js
📊 Size: 250 lines

🌳 AST Analysis Results:
─────────────────────────────────────────
✅ No security issues detected!

📋 Detailed Analysis:
─────────────────────────────────────────
  • AST parsing: ✅ Success
  • Source type: JavaScript
  • Total lines scanned: 250
  • Issues found: 0

═══════════════════════════════════════════
```

### Análisis de archivo con issues

```bash
fnpm ast-debug test-malicious.js
```

**Salida:**
```
🔍 AST Security Analysis
═══════════════════════════════════════════

📄 File: test-malicious.js
📊 Size: 23 lines

🌳 AST Analysis Results:
─────────────────────────────────────────
⚠️ Found 4 security issue(s)

Issue #1: ⚠️  WARNING
  Type: child_process_import
  Location: Line 12
  Description: child_process module imported - can execute system commands
  Code: require('child_process')

Issue #2: 🔴 CRITICAL
  Type: command_execution
  Location: Line 13
  Description: Command execution method 'exec' detected
  Code: cp.exec

Issue #3: 🔴 CRITICAL
  Type: eval_usage
  Location: Line 16
  Description: Direct eval() usage detected - allows arbitrary code execution
  Code: eval("console.log('dangerous')")

Issue #4: ⚠️  WARNING
  Type: dynamic_function
  Location: Line 19
  Description: Dynamic function creation with new Function() - potential code injection
  Code: new Function('return 1')

═══════════════════════════════════════════
```

## Casos de Uso

### 1. Verificar falsos positivos

Si un paquete es marcado como HIGH RISK pero crees que es un falso positivo:

```bash
fnpm ast-debug node_modules/<paquete>/lib/main.js
```

Esto te mostrará exactamente qué patrones están siendo detectados y por qué.

### 2. Comparar detección AST vs Regex

El comando muestra solo los resultados del análisis AST. Si el AST no detecta nada pero el scan completo sí, significa que el fallback de regex está detectando algo que el AST considera seguro.

### 3. Debugging de la detección

Si quieres entender por qué un archivo específico está siendo flaggeado:

```bash
fnpm ast-debug node_modules/<paquete>/archivo-sospechoso.js --verbose
```

El modo verbose muestra:
- Tipo de archivo detectado (JavaScript, TypeScript, ES Module, etc.)
- Número total de líneas escaneadas
- Cantidad de issues encontrados
- Estado del parsing AST

## Patrones Detectados

El análisis AST detecta los siguientes patrones:

### ✅ Detecta correctamente como PELIGROSO:
- `eval()` - Ejecución de código arbitrario
- `new Function()` - Creación dinámica de funciones
- `require('child_process')` - Importación de módulo de procesos
- `cp.exec()`, `cp.execSync()`, `cp.spawn()`, `cp.spawnSync()` - Ejecución de comandos del sistema
- Dynamic imports con rutas no literales

### ✅ Ignora correctamente como SEGURO:
- `/pattern/.exec()` - RegExp literal
- `simpleEncodingRegExp.exec()` - Variable con nombre relacionado a regex
- `new RegExp().exec()` - Constructor de RegExp
- `myPattern.exec()`, `urlMatch.exec()` - Variables con nombres descriptivos
- `eval()` dentro de strings o comentarios

## Limitaciones

El análisis AST puede fallar en:
- Código minificado/obfuscado
- Archivos con errores de sintaxis
- Características muy nuevas de JavaScript no soportadas

En estos casos, fnpm automáticamente usa el fallback de regex durante el scan normal.

## Diferencia con `fnpm scan`

- `fnpm scan` - Escanea todos los paquetes instalados, usa AST primero y fallback a regex
- `fnpm ast-debug` - Analiza un solo archivo, solo usa AST, útil para debugging

## Tips

1. **Verificar paquetes legítimos**: Si un paquete conocido como `webpack`, `babel`, o `express` es marcado como HIGH RISK, usa este comando para verificar si es un falso positivo.

2. **Entender el contexto**: El snippet de código mostrado te ayuda a entender el contexto exacto donde se detectó el patrón.

3. **Reportar issues**: Si encuentras falsos positivos, usa la salida de este comando para reportar el issue con contexto completo.
