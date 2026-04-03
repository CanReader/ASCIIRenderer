import { RenderParams } from '../hooks/useRenderer'

interface Props {
  params: RenderParams
  setParams: (p: Partial<RenderParams>) => void
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
      <span style={{ fontSize: 11, color: 'var(--text-dim)', flexShrink: 0, width: 80 }}>
        {label}
      </span>
      <div style={{ flex: 1 }}>{children}</div>
    </div>
  )
}

function Slider({
  label, value, min, max, step = 0.01, onChange, display,
}: {
  label: string; value: number; min: number; max: number;
  step?: number; onChange: (v: number) => void; display?: string
}) {
  return (
    <Row label={label}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <input type="range" min={min} max={max} step={step} value={value}
          onChange={e => onChange(parseFloat(e.target.value))} style={{ flex: 1 }} />
        <span style={{ fontSize: 11, color: 'var(--accent)', width: 46, textAlign: 'right', flexShrink: 0 }}>
          {display ?? value.toFixed(2)}
        </span>
      </div>
    </Row>
  )
}

function Toggle({ label, value, onChange }: {
  label: string; value: boolean; onChange: (v: boolean) => void
}) {
  return (
    <Row label={label}>
      <div
        className={`toggle-track ${value ? 'on' : ''}`}
        onClick={() => onChange(!value)}
        role="switch"
        aria-checked={value}
      >
        <div className="toggle-thumb" />
      </div>
    </Row>
  )
}

function Select({ label, value, options, onChange }: {
  label: string; value: string;
  options: { value: string; label: string }[];
  onChange: (v: string) => void
}) {
  return (
    <Row label={label}>
      <select value={value} onChange={e => onChange(e.target.value)} style={{ width: '100%' }}>
        {options.map(o => <option key={o.value} value={o.value}>{o.label}</option>)}
      </select>
    </Row>
  )
}

function Section({ title }: { title: string }) {
  return <div className="section-label">{title}</div>
}

export default function Controls({ params, setParams }: Props) {
  const deg = (r: number) => `${((r * 180) / Math.PI).toFixed(0)}°`

  return (
    <div>
      <Section title="display" />
      <Select label="charset" value={params.charset} onChange={v => setParams({ charset: v })}
        options={[
          { value: 'standard', label: 'standard (dense)' },
          { value: 'simple',   label: 'simple' },
          { value: 'blocks',   label: 'block chars' },
          { value: 'dots',     label: 'dots' },
          { value: 'binary',   label: 'binary 0/1' },
          { value: 'matrix',   label: 'matrix hex' },
        ]}
      />
      <Select label="shading" value={params.shading} onChange={v => setParams({ shading: v })}
        options={[
          { value: 'normal',    label: 'phong (default)' },
          { value: 'flat',      label: 'flat shading' },
          { value: 'depth',     label: 'depth buffer' },
          { value: 'normals',      label: 'normal map' },
          { value: 'texture',      label: 'texture' },
          { value: 'texture_lit',  label: 'texture + lit' },
          { value: 'wireframe',    label: 'wireframe' },
        ]}
      />
      <Select label="color" value={params.color_mode} onChange={v => setParams({ color_mode: v })}
        options={[
          { value: 'green',  label: 'phosphor green' },
          { value: 'amber',  label: 'amber' },
          { value: 'white',  label: 'white' },
          { value: 'blue',   label: 'cyan' },
          { value: 'red',    label: 'red' },
          { value: 'purple', label: 'purple' },
        ]}
      />
      <Toggle label="invert" value={params.invert} onChange={v => setParams({ invert: v })} />

      <Section title="resolution" />
      <Slider label="width" value={params.width} min={40} max={2000} step={2}
        onChange={v => setParams({ width: Math.round(v) })} display={`${params.width}ch`} />
      <Slider label="height" value={params.height} min={20} max={1000} step={2}
        onChange={v => setParams({ height: Math.round(v) })} display={`${params.height}ch`} />

      <Section title="camera" />
      <Slider label="zoom" value={params.zoom} min={0.1} max={6.0}
        onChange={v => setParams({ zoom: v })} display={`${params.zoom.toFixed(2)}×`} />

      <Section title="rotation" />
      <Toggle label="auto" value={params.auto_rotate} onChange={v => setParams({ auto_rotate: v })} />
      <Slider label="speed x" value={params.rotate_speed_x} min={-6} max={6}
        onChange={v => setParams({ rotate_speed_x: v })} />
      <Slider label="speed y" value={params.rotate_speed_y} min={-6} max={6}
        onChange={v => setParams({ rotate_speed_y: v })} />
      <Slider label="speed z" value={params.rotate_speed_z} min={-6} max={6}
        onChange={v => setParams({ rotate_speed_z: v })} />

      {!params.auto_rotate && (
        <>
          <Slider label="rot x" value={params.rot_x} min={-Math.PI} max={Math.PI}
            onChange={v => setParams({ rot_x: v })} display={deg(params.rot_x)} />
          <Slider label="rot y" value={params.rot_y} min={-Math.PI} max={Math.PI}
            onChange={v => setParams({ rot_y: v })} display={deg(params.rot_y)} />
          <Slider label="rot z" value={params.rot_z} min={-Math.PI} max={Math.PI}
            onChange={v => setParams({ rot_z: v })} display={deg(params.rot_z)} />
        </>
      )}

      <Section title="lighting" />
      <Slider label="ambient" value={params.ambient} min={0} max={1}
        onChange={v => setParams({ ambient: v })} />
      <Slider label="light x" value={params.light_x} min={-2} max={2}
        onChange={v => setParams({ light_x: v })} />
      <Slider label="light y" value={params.light_y} min={-2} max={2}
        onChange={v => setParams({ light_y: v })} />
      <Slider label="light z" value={params.light_z} min={-2} max={2}
        onChange={v => setParams({ light_z: v })} />
    </div>
  )
}
