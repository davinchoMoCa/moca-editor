// Genera el ícono de la app (1024x1024 PNG): fondo Monokai redondeado con "</>" en naranja.
// Uso: swift scripts/icon.swift <salida.png>
import AppKit

let size = 1024.0
let image = NSImage(size: NSSize(width: size, height: size))
image.lockFocus()
let inset = size * 0.1
let rect = NSRect(x: inset, y: inset, width: size - 2 * inset, height: size - 2 * inset)
NSColor(red: 0x27 / 255.0, green: 0x28 / 255.0, blue: 0x22 / 255.0, alpha: 1).setFill()
NSBezierPath(roundedRect: rect, xRadius: size * 0.18, yRadius: size * 0.18).fill()
let style = NSMutableParagraphStyle()
style.alignment = .center
let attrs: [NSAttributedString.Key: Any] = [
    .font: NSFont.monospacedSystemFont(ofSize: size * 0.3, weight: .bold),
    .foregroundColor: NSColor(red: 0xFD / 255.0, green: 0x97 / 255.0, blue: 0x1F / 255.0, alpha: 1),
    .paragraphStyle: style,
]
let text = NSAttributedString(string: "</>", attributes: attrs)
let h = text.size().height
text.draw(in: NSRect(x: 0, y: (size - h) / 2, width: size, height: h))
image.unlockFocus()
let png = NSBitmapImageRep(data: image.tiffRepresentation!)!.representation(using: .png, properties: [:])!
try! png.write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
