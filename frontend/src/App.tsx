import { useState, useEffect } from 'react'
import AsciiDisplay from './components/AsciiDisplay'
import FileUpload from './components/FileUpload'
import Controls from './components/Controls'
import { useRenderer } from './hooks/useRenderer'

type Tab = 'models' | 'render' | 'info'

const STATUS_COLOR: Record<string, string> = {
  idle:       'var(--text-dim)',
  connecting: 'var(--warn)',
  ready:      'var(--accent)',
  error:      'var(--err)',
}

const STATUS_LABEL: Record<string, string> = {
  idle:       'idle',
  connecting: 'connecting...',
  ready:      'streaming',
  error:      'error',
}

export default function App() {
  const [tab, setTab] = useState<Tab>('models')
  const [panelOpen, setPanelOpen] = useState(true)
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null)

  const renderer = useRenderer()

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement).tagName
      if (tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA') return
      switch (e.key) {
        case ' ':
          e.preventDefault()
          renderer.setParams({ auto_rotate: !renderer.params.auto_rotate })
          break
        case 'r': case 'R':
          renderer.setParams({ rot_x: 0, rot_y: 0, rot_z: 0, zoom: 1.0 })
          break
        case 'w': case 'W':
          renderer.setParams({ shading: renderer.params.shading === 'wireframe' ? 'normal' : 'wireframe' })
          break
        case '+': case '=':
          renderer.setParams({ zoom: Math.min(6.0, renderer.params.zoom * 1.15) })
          break
        case '-':
          renderer.setParams({ zoom: Math.max(0.1, renderer.params.zoom * 0.87) })
          break
        case 'Tab':
          e.preventDefault()
          setPanelOpen(v => !v)
          break
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [renderer])

  const handleSelectModel = (id: string) => {
    setSelectedModelId(id)
    renderer.connect(id)
    setTab('render')
  }

  const statusColor = STATUS_COLOR[renderer.status] ?? STATUS_COLOR.idle

  return (
    <div style={{ height: '100vh', width: '100vw', display: 'flex', flexDirection: 'column', overflow: 'hidden', background: 'var(--bg)' }}>

      {/* ── Header ────────────────────────────────────────────── */}
      <header style={{
        height: 32,
        display: 'flex',
        alignItems: 'center',
        borderBottom: '1px solid var(--border)',
        padding: '0 12px',
        gap: 16,
        flexShrink: 0,
        background: 'var(--bg-panel)',
      }}>
        {/* Logo */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ color: 'var(--accent)', fontSize: 12, fontWeight: 600, letterSpacing: '0.08em' }}>
            ASCII3D
          </span>
          <span style={{ color: 'var(--text-dim)', fontSize: 11 }}>renderer</span>
        </div>

        <span style={{ color: 'var(--border)', fontSize: 10 }}>│</span>

        {/* Status */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 5 }}>
          <span
            style={{ width: 5, height: 5, background: statusColor, display: 'inline-block',
              boxShadow: renderer.status === 'ready' ? `0 0 6px ${statusColor}` : 'none' }}
            className={renderer.status === 'connecting' ? 'status-connecting' : ''}
          />
          <span style={{ fontSize: 11, color: statusColor }}>
            {STATUS_LABEL[renderer.status]}
          </span>
        </div>

        {/* Perf stats */}
        {renderer.status === 'ready' && (
          <>
            <span style={{ color: 'var(--border)', fontSize: 10 }}>│</span>
            <span style={{ fontSize: 11, color: 'var(--text-dim)' }}>
              <span style={{ color: 'var(--accent)' }}>{renderer.fps}</span> fps
            </span>
            <span style={{ fontSize: 11, color: 'var(--text-dim)' }}>
              <span style={{ color: 'var(--text)' }}>{renderer.renderMs}</span> ms
            </span>
            {renderer.modelInfo && (
              <>
                <span style={{ color: 'var(--border)', fontSize: 10 }}>│</span>
                <span style={{ fontSize: 11, color: 'var(--text-dim)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', maxWidth: 200 }}>
                  {renderer.modelInfo.name}
                </span>
              </>
            )}
          </>
        )}

        {renderer.error && (
          <span style={{ fontSize: 11, color: 'var(--err)', marginLeft: 8 }}>
            ✕ {renderer.error}
          </span>
        )}

        {/* Spacer + hints */}
        <div style={{ flex: 1 }} />
        <span style={{ fontSize: 10, color: 'var(--text-dim)', letterSpacing: '0.05em' }}>
          tab · hide panel
        </span>
      </header>

      {/* ── Body ──────────────────────────────────────────────── */}
      <div style={{ flex: 1, display: 'flex', minHeight: 0, position: 'relative' }}>

        {/* ── Side panel ──────────────────────────────────────── */}
        <div style={{
          width: panelOpen ? 260 : 0,
          minWidth: panelOpen ? 260 : 0,
          overflow: 'hidden',
          borderRight: panelOpen ? '1px solid var(--border)' : 'none',
          background: 'var(--bg-panel)',
          display: 'flex',
          flexDirection: 'column',
          transition: 'width 0.15s, min-width 0.15s',
          flexShrink: 0,
        }}>
          <div style={{ width: 260, display: 'flex', flexDirection: 'column', height: '100%' }}>

            {/* Tab bar */}
            <div style={{
              display: 'flex',
              borderBottom: '1px solid var(--border)',
              flexShrink: 0,
            }}>
              {(['models', 'render', 'info'] as Tab[]).map(t => (
                <button
                  key={t}
                  onClick={() => setTab(t)}
                  style={{
                    flex: 1,
                    padding: '6px 0',
                    background: 'none',
                    border: 'none',
                    borderBottom: tab === t ? '1px solid var(--accent)' : '1px solid transparent',
                    color: tab === t ? 'var(--accent)' : 'var(--text-dim)',
                    fontSize: 11,
                    cursor: 'pointer',
                    letterSpacing: '0.08em',
                    marginBottom: -1,
                    transition: 'color 0.1s',
                    fontFamily: 'inherit',
                  }}
                >
                  {t}
                </button>
              ))}
            </div>

            {/* Tab content */}
            <div style={{ flex: 1, overflowY: 'auto', padding: '10px 12px' }}>
              {tab === 'models' && (
                <FileUpload
                  onSelectModel={handleSelectModel}
                  selectedModelId={selectedModelId}
                />
              )}
              {tab === 'render' && (
                <Controls params={renderer.params} setParams={renderer.setParams} />
              )}
              {tab === 'info' && (
                <InfoPanel
                  modelInfo={renderer.modelInfo}
                  fps={renderer.fps}
                  renderMs={renderer.renderMs}
                  params={renderer.params}
                />
              )}
            </div>

          </div>
        </div>

        {/* ── ASCII canvas ─────────────────────────────────────── */}
        <div style={{ flex: 1, position: 'relative', minWidth: 0 }}>
          <AsciiDisplay
            frame={renderer.frame}
            colors={renderer.colors}
            colorMode={renderer.params.color_mode}
            isDragging={renderer.isDragging}
            onMouseDown={renderer.onMouseDown}
            onMouseMove={renderer.onMouseMove}
            onMouseUp={renderer.onMouseUp}
            onWheel={renderer.onWheel}
          />

          {/* Minimal overlay hints */}
          {renderer.status === 'ready' && (
            <div style={{
              position: 'absolute', bottom: 8, right: 10,
              fontSize: 10, color: 'var(--text-dim)',
              pointerEvents: 'none', textAlign: 'right', lineHeight: 1.8,
            }}>
              <span>drag · rotate</span><br />
              <span>scroll · zoom</span><br />
              <span>space · {renderer.params.auto_rotate ? 'pause' : 'play'}</span>
            </div>
          )}
        </div>

      </div>

      {/* ── Status bar ────────────────────────────────────────── */}
      <footer style={{
        height: 20,
        borderTop: '1px solid var(--border)',
        display: 'flex',
        alignItems: 'center',
        padding: '0 12px',
        gap: 16,
        flexShrink: 0,
        background: 'var(--bg-panel)',
      }}>
        {renderer.modelInfo ? (
          <>
            <Stat label="verts" value={renderer.modelInfo.vertex_count.toLocaleString()} />
            <Stat label="faces" value={renderer.modelInfo.face_count.toLocaleString()} />
            <Stat label="res" value={`${renderer.params.width}×${renderer.params.height}`} />
            <Stat label="charset" value={renderer.params.charset} />
            <Stat label="shading" value={renderer.params.shading} />
          </>
        ) : (
          <span style={{ fontSize: 10, color: 'var(--text-dim)' }}>
            no model loaded — drop a .obj / .gltf / .glb file
          </span>
        )}
        <div style={{ flex: 1 }} />
        <span style={{ fontSize: 10, color: 'var(--text-dim)', letterSpacing: '0.05em' }}>
          w · wireframe &nbsp; r · reset &nbsp; +/- · zoom
        </span>
      </footer>
    </div>
  )
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <span style={{ fontSize: 10, color: 'var(--text-dim)' }}>
      {label}:{' '}
      <span style={{ color: 'var(--text)' }}>{value}</span>
    </span>
  )
}

function InfoPanel({
  modelInfo, fps, renderMs, params,
}: {
  modelInfo: { name: string; vertex_count: number; face_count: number } | null
  fps: number; renderMs: number
  params: { width: number; height: number; charset: string; shading: string }
}) {
  return (
    <div>
      <div className="section-label">model</div>
      {modelInfo ? (
        <>
          <InfoRow k="file"     v={modelInfo.name} />
          <InfoRow k="vertices" v={modelInfo.vertex_count.toLocaleString()} />
          <InfoRow k="faces"    v={modelInfo.face_count.toLocaleString()} />
        </>
      ) : (
        <p style={{ fontSize: 11, color: 'var(--text-dim)', margin: '6px 0' }}>no model loaded</p>
      )}

      <div className="section-label">performance</div>
      <InfoRow k="fps"       v={`${fps}`} accent />
      <InfoRow k="render"    v={`${renderMs} ms/frame`} />
      <InfoRow k="res"       v={`${params.width} × ${params.height} chars`} />
      <InfoRow k="charset"   v={params.charset} />
      <InfoRow k="shading"   v={params.shading} />

      <div className="section-label">shortcuts</div>
      <div style={{ fontSize: 11, color: 'var(--text-dim)', lineHeight: 2 }}>
        <KBD k="tab" /> hide/show panel<br />
        <KBD k="space" /> toggle auto-rotate<br />
        <KBD k="r" /> reset view<br />
        <KBD k="w" /> wireframe toggle<br />
        <KBD k="+/-" /> zoom in/out<br />
        <KBD k="drag" /> rotate model<br />
        <KBD k="scroll" /> zoom
      </div>
    </div>
  )
}

function InfoRow({ k, v, accent }: { k: string; v: string; accent?: boolean }) {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 11, marginBottom: 3 }}>
      <span style={{ color: 'var(--text-dim)' }}>{k}</span>
      <span style={{ color: accent ? 'var(--accent)' : 'var(--text)' }}>{v}</span>
    </div>
  )
}

function KBD({ k }: { k: string }) {
  return (
    <span style={{
      display: 'inline-block',
      background: 'var(--bg-input)',
      border: '1px solid var(--border)',
      padding: '0 4px',
      fontSize: 10,
      color: 'var(--text)',
      minWidth: 36,
      textAlign: 'center',
      letterSpacing: '0.03em',
    }}>
      {k}
    </span>
  )
}
