import { useEffect, useRef } from 'react'

const CRT_CLASS: Record<string, string> = {
  green:  '',
  amber:  'crt-amber',
  white:  'crt-white',
  blue:   'crt-blue',
  red:    'crt-red',
  purple: 'crt-purple',
}

interface Props {
  frame: string
  colors: string | null
  colorMode: string
  isDragging: boolean
  onMouseDown: (e: React.MouseEvent) => void
  onMouseMove: (e: React.MouseEvent) => void
  onMouseUp: () => void
  onWheel: (e: React.WheelEvent) => void
}

export default function AsciiDisplay({
  frame, colors, colorMode, isDragging,
  onMouseDown, onMouseMove, onMouseUp, onWheel,
}: Props) {
  const preRef = useRef<HTMLPreElement>(null)
  const canvasRef = useRef<HTMLCanvasElement>(null)

  // Pre rendering — used when no per-cell colors
  useEffect(() => {
    if (colors !== null) return
    const el = preRef.current
    if (!el || !frame) return
    const lines = frame.split('\n')
    const cols = lines[0]?.length ?? 0
    const rows = lines.length
    if (cols === 0 || rows === 0) return
    const parent = el.parentElement
    if (!parent) return
    const containerW = parent.clientWidth - 32
    const containerH = parent.clientHeight - 32
    const fontW = containerW / cols
    const fontH = containerH / rows
    const fontSize = Math.min(fontW / 0.6, fontH)
    el.style.fontSize = `${Math.max(5, Math.min(20, fontSize))}px`
  }, [frame, colors])

  // Canvas rendering — used when per-cell colors are present
  useEffect(() => {
    if (colors === null) return
    const canvas = canvasRef.current
    if (!canvas || !frame) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    const lines = frame.split('\n').filter(l => l.length > 0)
    const rows = lines.length
    const cols = lines[0]?.length ?? 0
    if (rows === 0 || cols === 0) return

    const parent = canvas.parentElement
    if (!parent) return
    const containerW = parent.clientWidth
    const containerH = parent.clientHeight

    // Match the <pre> font sizing logic
    const fontW = (containerW - 32) / cols
    const fontH = (containerH - 32) / rows
    const fontSize = Math.max(5, Math.min(20, Math.min(fontW / 0.6, fontH)))

    canvas.width = containerW
    canvas.height = containerH

    ctx.clearRect(0, 0, canvas.width, canvas.height)
    ctx.font = `${fontSize}px "JetBrains Mono", "Fira Code", monospace`
    ctx.textBaseline = 'top'

    const charW = fontSize * 0.6
    const charH = fontSize

    const totalW = cols * charW
    const totalH = rows * charH
    const offsetX = (containerW - totalW) / 2
    const offsetY = (containerH - totalH) / 2

    for (let row = 0; row < rows; row++) {
      const line = lines[row]
      for (let col = 0; col < line.length; col++) {
        const ch = line[col]
        if (ch === ' ') continue

        const cellIdx = row * cols + col
        const hexOffset = cellIdx * 6
        const hex = colors.slice(hexOffset, hexOffset + 6)
        if (hex.length < 6) continue

        const r = parseInt(hex.slice(0, 2), 16)
        const g = parseInt(hex.slice(2, 4), 16)
        const b = parseInt(hex.slice(4, 6), 16)

        ctx.fillStyle = `rgb(${r},${g},${b})`
        ctx.fillText(ch, offsetX + col * charW, offsetY + row * charH)
      }
    }
  }, [frame, colors])

  const crtClass = colors !== null ? '' : (CRT_CLASS[colorMode] ?? '')

  return (
    <div
      className={`crt-display w-full h-full flex items-center justify-center ${crtClass}`}
      style={{ cursor: isDragging ? 'crosshair' : 'default' }}
      onMouseDown={onMouseDown}
      onMouseMove={onMouseMove}
      onMouseUp={onMouseUp}
      onMouseLeave={onMouseUp}
      onWheel={onWheel}
    >
      {frame ? (
        colors !== null ? (
          <canvas
            ref={canvasRef}
            style={{ display: 'block', width: '100%', height: '100%' }}
          />
        ) : (
          <pre
            ref={preRef}
            className="crt-text select-none whitespace-pre leading-none"
            style={{ fontFamily: "'JetBrains Mono', 'Fira Code', monospace" }}
          >
            {frame}
          </pre>
        )
      ) : (
        <div className="relative z-10 text-center" style={{ color: 'var(--text-dim)' }}>
          <pre style={{ fontSize: 11, lineHeight: 1.2, color: 'var(--accent-dim)', letterSpacing: '0.05em' }}>{`
  ╔══════════════════════════╗
  ║   ASCII 3D RENDERER      ║
  ║                          ║
  ║   DROP A MODEL TO BEGIN  ║
  ║                          ║
  ║   .obj  .gltf  .glb      ║
  ╚══════════════════════════╝`}
          </pre>
          <p style={{ fontSize: 11, marginTop: 12, color: 'var(--text-dim)' }}>
            drag to rotate · scroll to zoom
          </p>
        </div>
      )}
    </div>
  )
}
