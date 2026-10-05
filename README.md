# Moca Editor

Editor de código ligero tipo Sublime Text, escrito en Rust con [egui/eframe](https://github.com/emilk/egui).

- Pestañas, árbol de archivos (solo al abrir una carpeta), resaltado de sintaxis (syntect)
- Abrir rápido con búsqueda difusa (⌘P), buscar y reemplazar (⌘F)
- Tema Monokai, espaciado y menús según Material Design 3, íconos Material Symbols
- Archivos grandes: el resaltado de sintaxis se limita a una ventana alrededor de lo visible

## Uso

```bash
cargo run --release -- [carpeta|archivo]...
```

Sin argumentos abre como editor de texto plano.

### Instalar como app (macOS)

```bash
scripts/bundle-macos.sh            # compila, arma "Moca Editor.app", la firma y la copia a /Applications
scripts/bundle-macos.sh <identidad> # firmar con otro certificado (ver: security find-identity -v -p codesigning)
```

| Atajo | Acción |
|---|---|
| ⌘N / ⌘O / ⇧⌘O | Nuevo / abrir archivo / abrir carpeta |
| ⌘S / ⌘W | Guardar / cerrar pestaña |
| ⌘P / ⌘F | Abrir rápido / buscar |
| ⌃` | Abrir Terminal en la raíz del proyecto (macOS) |

## Estado

Prototipo en desarrollo. Pendiente: aviso de cambios sin guardar, abrir archivos con doble clic
desde Finder, sangría automática.

## Créditos

Construido con Claude Code como orquestador y modelos GLM / Gemma (vía OpenCode) como ejecutores.

`assets/MaterialSymbolsOutlined-subset.ttf` es un subconjunto de
[Material Symbols](https://github.com/google/material-design-icons) de Google, bajo licencia
Apache 2.0.
