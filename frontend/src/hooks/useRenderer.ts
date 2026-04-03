import { useCallback, useEffect, useRef, useState } from 'react'
import type { RenderParams } from '../bindings/RenderParams'

export type { RenderParams }

export const DEFAULT_PARAMS: RenderParams = {
  width: 160,
  height: 60,
  rot_x: 0,
  rot_y: 0,
  rot_z: 0,
  auto_rotate: true,
  rotate_speed_x: 0,
  rotate_speed_y: 1.0,
  rotate_speed_z: 0,
  zoom: 1.0,
  charset: 'standard',
  shading: 'normal',
  light_x: 0.5,
  light_y: 1.0,
  light_z: 0.8,
  ambient: 0.15,
  invert: false,
  color_mode: 'green',
}

export interface ModelInfo {
  name: string
  vertex_count: number
  face_count: number
}

export type RendererStatus = 'idle' | 'connecting' | 'ready' | 'error'

export interface RendererState {
  frame: string
  colors: string | null
  fps: number
  renderMs: number
  status: RendererStatus
  error: string | null
  modelInfo: ModelInfo | null
  params: RenderParams
  setParams: (p: Partial<RenderParams>) => void
  connect: (modelId: string) => void
  disconnect: () => void
  isDragging: boolean
  onMouseDown: (e: React.MouseEvent) => void
  onMouseMove: (e: React.MouseEvent) => void
  onMouseUp: () => void
  onWheel: (e: React.WheelEvent) => void
}

export function useRenderer(): RendererState {
  const [frame, setFrame] = useState('')
  const [colors, setColors] = useState<string | null>(null)
  const [fps, setFps] = useState(0)
  const [renderMs, setRenderMs] = useState(0)
  const [status, setStatus] = useState<RendererStatus>('idle')
  const [error, setError] = useState<string | null>(null)
  const [modelInfo, setModelInfo] = useState<ModelInfo | null>(null)
  const [params, setParamsState] = useState<RenderParams>(DEFAULT_PARAMS)
  const [isDragging, setIsDragging] = useState(false)

  const wsRef = useRef<WebSocket | null>(null)
  const paramsRef = useRef<RenderParams>(DEFAULT_PARAMS)
  const fpsCounterRef = useRef({ count: 0, lastTime: Date.now() })
  const dragRef = useRef<{ x: number; y: number } | null>(null)
  const pendingParamUpdate = useRef(false)

  // Keep paramsRef in sync
  useEffect(() => {
    paramsRef.current = params
  }, [params])

  const sendParams = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(
        JSON.stringify({ type: 'update_params', params: paramsRef.current })
      )
      pendingParamUpdate.current = false
    }
  }, [])

  const setParams = useCallback((updates: Partial<RenderParams>) => {
    setParamsState(prev => {
      const next = { ...prev, ...updates }
      paramsRef.current = next
      if (!pendingParamUpdate.current) {
        pendingParamUpdate.current = true
        requestAnimationFrame(() => sendParams())
      }
      return next
    })
  }, [sendParams])

  const connect = useCallback((modelId: string) => {
    if (wsRef.current) {
      wsRef.current.close()
    }

    setStatus('connecting')
    setError(null)
    setFrame('')

    const ws = new WebSocket(`ws://${window.location.host}/ws/render`)
    wsRef.current = ws

    ws.onopen = () => {
      ws.send(JSON.stringify({
        type: 'init',
        model_id: modelId,
        width: paramsRef.current.width,
        height: paramsRef.current.height,
      }))
    }

    ws.onmessage = (ev) => {
      const msg = JSON.parse(ev.data)

      switch (msg.type) {
        case 'frame':
          setFrame(msg.content)
          setColors(msg.colors ?? null)
          setRenderMs(msg.elapsed_ms)
          // FPS calculation
          fpsCounterRef.current.count++
          const now = Date.now()
          const elapsed = now - fpsCounterRef.current.lastTime
          if (elapsed >= 500) {
            setFps(Math.round((fpsCounterRef.current.count / elapsed) * 1000))
            fpsCounterRef.current = { count: 0, lastTime: now }
          }
          break
        case 'model_info':
          setModelInfo({
            name: msg.name,
            vertex_count: msg.vertex_count,
            face_count: msg.face_count,
          })
          break
        case 'ready':
          setStatus('ready')
          sendParams()
          break
        case 'error':
          setError(msg.message)
          setStatus('error')
          break
      }
    }

    ws.onerror = () => {
      setError('WebSocket connection failed')
      setStatus('error')
    }

    ws.onclose = () => {
      if (status !== 'error') {
        setStatus('idle')
      }
    }
  }, [sendParams, status])

  const disconnect = useCallback(() => {
    wsRef.current?.close()
    wsRef.current = null
    setStatus('idle')
    setFrame('')
    setColors(null)
    setModelInfo(null)
  }, [])

  // Mouse drag for manual rotation
  const onMouseDown = useCallback((e: React.MouseEvent) => {
    if (e.button !== 0) return
    dragRef.current = { x: e.clientX, y: e.clientY }
    setIsDragging(true)
    setParams({ auto_rotate: false })
  }, [setParams])

  const onMouseMove = useCallback((e: React.MouseEvent) => {
    if (!dragRef.current) return
    const dx = e.clientX - dragRef.current.x
    const dy = e.clientY - dragRef.current.y
    dragRef.current = { x: e.clientX, y: e.clientY }

    const sensitivity = 0.01
    setParams({
      rot_y: paramsRef.current.rot_y + dx * sensitivity,
      rot_x: paramsRef.current.rot_x + dy * sensitivity,
    })
  }, [setParams])

  const onMouseUp = useCallback(() => {
    dragRef.current = null
    setIsDragging(false)
  }, [])

  const onWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault()
    const delta = e.deltaY > 0 ? 0.9 : 1.1
    setParams({ zoom: Math.max(0.1, Math.min(5.0, paramsRef.current.zoom * delta)) })
  }, [setParams])

  useEffect(() => {
    return () => { wsRef.current?.close() }
  }, [])

  return {
    frame, colors, fps, renderMs, status, error, modelInfo, params,
    setParams, connect, disconnect, isDragging,
    onMouseDown, onMouseMove, onMouseUp, onWheel,
  }
}
