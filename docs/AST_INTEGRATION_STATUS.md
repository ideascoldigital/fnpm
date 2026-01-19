# Integración AST para Análisis de Seguridad

## Estado Actual

✅ **IMPLEMENTADO** - Hemos integrado análisis AST (Abstract Syntax Tree) usando **oxc** (Oxidation Compiler) para mejorar la precisión de la detección de seguridad.

## Decisión Tomada

Migración exitosa a `oxc` en lugar de `swc` por sus ventajas técnicas y mejor mantenimiento.

## Implementación Realizada

### Dependencias Instaladas

```toml
[dependencies]
oxc_parser = "0.38"
oxc_ast = "0.38"  
oxc_span = "0.38"
oxc_allocator = "0.38"
```

### Módulos Creados

1. **`src/ast_security_analyzer.rs`** - Analizador AST principal con oxc
   - `SecurityVisitor` - Visitor pattern para detectar patrones maliciosos
   - `analyze_js_file()` - Función pública para analizar archivos
   - `analyze_js_source()` - Función para analizar código fuente

2. **Integración en `src/security.rs`**
   - Modificado `scan_source_code()` para usar AST primero
   - Fallback a regex para código minificado/obfuscado
   - Soporte para `.js`, `.mjs`, `.cjs`, `.ts`, `.tsx`

### Patrones Detectados por AST

✅ **Implementado:**
- `eval()` - Detección precisa (ignora strings y comentarios)
- `new Function()` - Creación dinámica de funciones
- Dynamic imports - Imports con rutas no literales
- `child_process` - Importación y uso del módulo
- Command execution - Métodos `exec`, `execSync`, `spawn`, `spawnSync`
  - ✅ **Distingue entre `RegExp.exec()` (seguro) y `child_process.exec()` (peligroso)**
  - Detecta contexto de RegExp literal: `/pattern/.exec()`
  - Detecta contexto de new RegExp: `new RegExp().exec()`
  - Identifica variables con nombres relacionados a regex:
    - `simpleEncodingRegExp` (negotiator)
    - `myPattern`, `urlMatch`, `testRe`
    - Cualquier variable con "regex", "regexp", "pattern", "match" en el nombre
  - ✅ **Rastreo de asignaciones de variables**:
    - `var e = RegExp.prototype; e.exec()` (Babel wrapRegExp)
    - Detecta cuando una variable contiene `RegExp.prototype`

## Beneficios del Análisis AST

Una vez implementado, el análisis AST proporcionará:

### 1. **Detección Precisa de eval()**
```javascript
// AST puede diferenciar:
eval("code")              // ❌ Flagged - eval real
console.log("eval()")     // ✅ Ignorado - string
// Comment: eval() here   // ✅ Ignorado - comentario
```

### 2. **Detección Precisa de new Function()**
```javascript
// AST puede diferenciar:
new Function('return 1')           // ⚠️  Warning - creación dinámica
getCreateFunction(313)             // ✅ Ignorado - llamada normal
function createJSDocType() { }     // ✅ Ignorado - declaración
```

### 3. **Análisis Contextual de require()**
```javascript
// AST puede analizar scope y contexto:
require("dayjs")                   // ✅ Ignorado - estático
require(basePath + "/module")      // ❌ Flagged - dinámico
module.exports = require("pkg")    // ✅ Ignorado - UMD pattern
```

### 4. **Detección de child_process vs RegExp** ✅ IMPLEMENTADO
```javascript
// AST sabe el tipo del objeto:
const cp = require('child_process');
cp.exec('ls')                      // ❌ Flagged - ejecución de comandos

const regex = /test/;
regex.exec(str)                    // ✅ Ignorado - método de RegExp

// Casos reales de paquetes npm:
const match = /^\/(.*)\/([yugi]*)$/.exec(value);  // ✅ Ignorado - webpack-dev-server

// negotiator package:
var simpleEncodingRegExp = /^\s*([^\s;]+)\s*(?:;(.*))?$/;
var match = simpleEncodingRegExp.exec(str);        // ✅ Ignorado - variable con "RegExp" en nombre

// @babel/runtime wrapRegExp helper:
var e = RegExp.prototype;
BabelRegExp.prototype.exec = function (r) {
    var t = e.exec.call(this, r);                  // ✅ Ignorado - e es RegExp.prototype
    return t;
};
```

### 5. **Análisis de Flujo de Datos**
```javascript
// AST puede seguir el flujo:
const malicious = atob('base64...');
eval(malicious);                   // 🔴 CRITICAL - obfuscación + eval

const template = "x + y";
new Function(template);            // ⚠️  Warning - legítimo
```

## Impacto Esperado

Con AST implementado, esperamos reducir falsos positivos en:

- **webpack**: ~95% reducción (de 11 issues a ~0)
- **TypeScript**: 100% reducción (ya en 0)
- **ejs**: ~80% reducción (de 15 issues a ~3)
- **Build tools**: ~90% reducción general

Mientras que **MANTENEMOS** la detección de:
- ✅ Supply chain attacks reales
- ✅ Code injection attempts
- ✅ Data exfiltration patterns
- ✅ Behavioral attack chains

## Próximos Pasos

### Mejoras Futuras

1. **Expandir patrones de detección**
   - Análisis de flujo de datos (data flow analysis)
   - Detección de obfuscación más sofisticada
   - Tracking de variables sospechosas

2. **Tests comprehensivos**
   - ✅ Tests básicos incluidos en el módulo
   - Agregar tests con código real de webpack, babel, etc.
   - Verificar reducción de falsos positivos

3. **Optimización de performance**
   - AST es más lento que regex
   - Considerar cache de resultados
   - Paralelización de análisis de múltiples archivos

4. **Métricas y validación**
   - Medir reducción de falsos positivos
   - Validar con paquetes conocidos (webpack, TypeScript, etc.)
   - Documentar casos edge

## Tests Incluidos

El módulo incluye tests unitarios para validar:
- ✅ Detección de `eval()`
- ✅ Detección de `new Function()`
- ✅ Ignorar `eval()` en strings
- ✅ Detección de `child_process`
- ✅ Dynamic imports
- ✅ Static imports (no deben flaggearse)
- ✅ `RegExp.exec()` no se detecta como command execution
- ✅ `child_process.exec()` sí se detecta correctamente
- ✅ `new RegExp().exec()` es reconocido como seguro
- ✅ `simpleEncodingRegExp.exec()` (negotiator) es reconocido como seguro
- ✅ Variables con nombres como `pattern`, `match`, `regex` son reconocidas
- ✅ `var e = RegExp.prototype; e.exec()` (Babel) es reconocido como seguro
- ✅ Rastreo de asignaciones de variables a `RegExp.prototype`

## Uso

El análisis AST se ejecuta automáticamente cuando se escanea un paquete:

```bash
fnpm scan <package-name>
```

El sistema usa AST como método principal y solo cae en regex si:
- ❌ El archivo tiene errores de sintaxis
- ❌ El código está minificado u obfuscado  
- ❌ Hay características de JavaScript no soportadas por oxc

**Importante**: Si el AST funciona correctamente, **NO se usa regex**, incluso si no encuentra issues. Esto previene falsos positivos del regex (como detectar `eval()` en comentarios).
