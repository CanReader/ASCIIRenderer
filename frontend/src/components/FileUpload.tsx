import { useCallback, useEffect, useRef, useState } from 'react'

interface UploadedModel {
  id: string
  name: string
  vertex_count: number
  face_count: number
  is_example: boolean
}

interface Props {
  onSelectModel: (id: string) => void
  selectedModelId: string | null
}

export default function FileUpload({ onSelectModel, selectedModelId }: Props) {
  const [exampleModels, setExampleModels] = useState<UploadedModel[]>([])
  const [uploadedModels, setUploadedModels] = useState<UploadedModel[]>([])
  const [uploading, setUploading] = useState(false)
  const [progress, setProgress] = useState('')
  const [uploadError, setUploadError] = useState<string | null>(null)
  const [dragOver, setDragOver] = useState(false)
  const fileInputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    fetch('/api/models')
      .then(res => res.json())
      .then((data: UploadedModel[]) => {
        setExampleModels(data.filter(m => m.is_example))
        setUploadedModels(data.filter(m => !m.is_example))
      })
      .catch(() => { /* silently ignore — backend may not be ready yet */ })
  }, [])

  const uploadFile = useCallback(async (file: File) => {
    const ext = file.name.split('.').pop()?.toLowerCase()
    if (!ext || !['obj', 'gltf', 'glb', 'fbx'].includes(ext)) {
      setUploadError(`ERR: unsupported format ".${ext}"`)
      return
    }

    setUploading(true)
    setUploadError(null)
    setProgress(`uploading ${file.name} (${(file.size / 1024).toFixed(0)} kb)...`)

    const form = new FormData()
    form.append('file', file, file.name)

    try {
      const res = await fetch('/api/upload', { method: 'POST', body: form })
      const text = await res.text()
      if (!text) throw new Error('Empty response from server — is the backend running?')
      const data = JSON.parse(text)
      if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`)
      setUploadedModels(prev => [data as UploadedModel, ...prev])
      onSelectModel(data.id)
      setProgress('')
    } catch (err) {
      setUploadError(`ERR: ${(err as Error).message}`)
      setProgress('')
    } finally {
      setUploading(false)
    }
  }, [onSelectModel])

  const onFileChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (file) uploadFile(file)
    e.target.value = ''
  }, [uploadFile])

  const onDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    setDragOver(false)
    const file = e.dataTransfer.files[0]
    if (file) uploadFile(file)
  }, [uploadFile])

  const deleteModel = useCallback(async (id: string, e: React.MouseEvent) => {
    e.stopPropagation()
    await fetch(`/api/models/${id}`, { method: 'DELETE' })
    setUploadedModels(prev => prev.filter(m => m.id !== id))
  }, [])

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
      {exampleModels.length > 0 && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
          <p style={{ fontSize: 10, color: 'var(--text-dim)', letterSpacing: '0.12em', textTransform: 'uppercase', margin: '0 0 4px' }}>
            ▸ examples
          </p>
          {exampleModels.map(model => (
            <div
              key={model.id}
              onClick={() => onSelectModel(model.id)}
              className={`model-item ${selectedModelId === model.id ? 'selected' : ''}`}
            >
              {selectedModelId === model.id && <span className="item-indicator" />}
              <div style={{ flex: 1, minWidth: 0 }}>
                <p style={{ fontSize: 11, margin: 0, color: 'var(--text-bright)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                  {model.name}
                </p>
                <p style={{ fontSize: 10, margin: '1px 0 0', color: 'var(--text-dim)' }}>
                  {model.vertex_count.toLocaleString()} verts · {model.face_count.toLocaleString()} faces
                </p>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Drop zone */}
      <div
        onDragOver={e => { e.preventDefault(); setDragOver(true) }}
        onDragLeave={() => setDragOver(false)}
        onDrop={onDrop}
        onClick={() => !uploading && fileInputRef.current?.click()}
        className={`drop-zone ${dragOver ? 'active' : ''}`}
      >
        <input
          ref={fileInputRef}
          type="file"
          accept=".obj,.gltf,.glb,.fbx"
          style={{ display: 'none' }}
          onChange={onFileChange}
        />
        {uploading ? (
          <p style={{ fontSize: 11, color: 'var(--accent)', margin: 0 }}>
            {progress}
          </p>
        ) : (
          <>
            <p style={{ fontSize: 11, color: 'var(--text-dim)', margin: '0 0 4px' }}>
              [ DROP FILE OR CLICK ]
            </p>
            <p style={{ fontSize: 10, color: '#1e1e34', margin: 0 }}>
              .obj · .gltf · .glb · .fbx
            </p>
          </>
        )}
      </div>

      {uploadError && (
        <p style={{ fontSize: 11, color: 'var(--err)', margin: 0, padding: '0 2px' }}>
          {uploadError}
        </p>
      )}

      {uploadedModels.length > 0 && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 3, marginTop: 4 }}>
          <p style={{ fontSize: 10, color: 'var(--text-dim)', letterSpacing: '0.12em', textTransform: 'uppercase', margin: '0 0 4px' }}>
            ▸ loaded models
          </p>
          {uploadedModels.map(model => (
            <div
              key={model.id}
              onClick={() => onSelectModel(model.id)}
              className={`model-item ${selectedModelId === model.id ? 'selected' : ''}`}
            >
              {selectedModelId === model.id && <span className="item-indicator" />}
              <div style={{ flex: 1, minWidth: 0 }}>
                <p style={{ fontSize: 11, margin: 0, color: 'var(--text-bright)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                  {model.name}
                </p>
                <p style={{ fontSize: 10, margin: '1px 0 0', color: 'var(--text-dim)' }}>
                  {model.vertex_count.toLocaleString()} verts · {model.face_count.toLocaleString()} faces
                </p>
              </div>
              <button
                onClick={(e) => deleteModel(model.id, e)}
                style={{
                  background: 'none', border: 'none', cursor: 'pointer',
                  color: 'var(--text-dim)', fontSize: 11, padding: '0 2px',
                  opacity: 0, transition: 'opacity 0.1s',
                }}
                onMouseEnter={e => (e.currentTarget.style.opacity = '1')}
                onMouseLeave={e => (e.currentTarget.style.opacity = '0')}
                title="Remove"
              >
                ✕
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
