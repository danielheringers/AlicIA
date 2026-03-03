"use client"

import { ArrowUp, Paperclip, ImagePlus, AtSign, X } from "lucide-react"
import { useState, useRef, useEffect, useCallback } from "react"
import { CommandPalette } from "./command-palette"
import type { RuntimeMethodCapabilities } from "@/lib/application/runtime-types"

interface CommandInputProps {
  onSubmit: (value: string) => Promise<void> | void
  onSlashCommand: (command: string) => Promise<void> | void
  onAttachImage?: () => Promise<void> | void
  onAttachMention?: () => Promise<void> | void
  onRemoveImage?: (index: number) => void
  onRemoveMention?: (index: number) => void
  pendingImages?: string[]
  pendingMentions?: string[]
  runtimeCapabilities: RuntimeMethodCapabilities
  disabled?: boolean
}

function fileLabel(path: string): string {
  const normalized = path.replace(/\\/g, "/")
  const segments = normalized.split("/").filter(Boolean)
  return segments.at(-1) ?? path
}

export function CommandInput({
  onSubmit,
  onSlashCommand,
  onAttachImage,
  onAttachMention,
  onRemoveImage,
  onRemoveMention,
  pendingImages = [],
  pendingMentions = [],
  runtimeCapabilities,
  disabled,
}: CommandInputProps) {
  const [value, setValue] = useState("")
  const [showPalette, setShowPalette] = useState(false)
  const [paletteFilter, setPaletteFilter] = useState("")
  const [isSubmitting, setIsSubmitting] = useState(false)
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto"
      textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 200)}px`
    }
  }, [value])

  useEffect(() => {
    if (value.startsWith("/")) {
      setShowPalette(true)
      setPaletteFilter(value)
    } else {
      setShowPalette(false)
      setPaletteFilter("")
    }
  }, [value])

  const handleSubmit = useCallback(async () => {
    if (isSubmitting || disabled) {
      return
    }

    const hasText = value.trim().length > 0
    const hasAttachments = pendingImages.length > 0 || pendingMentions.length > 0

    if (!hasText && !hasAttachments) {
      return
    }

    setIsSubmitting(true)

    try {
      if (value.startsWith("/")) {
        await onSlashCommand(value.trim())
      } else {
        await onSubmit(value.trim())
      }
      setValue("")
      setShowPalette(false)
    } finally {
      setIsSubmitting(false)
    }
  }, [
    disabled,
    isSubmitting,
    onSlashCommand,
    onSubmit,
    pendingImages.length,
    pendingMentions.length,
    value,
  ])

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey && !showPalette) {
      e.preventDefault()
      void handleSubmit()
    }

    if (e.key === "Escape" && showPalette) {
      e.preventDefault()
      setShowPalette(false)
      setValue("")
    }
  }

  const handleCommandSelect = (command: string) => {
    setShowPalette(false)
    setValue("")
    void onSlashCommand(command)
  }

  const getPalettePosition = () => {
    if (!containerRef.current) {
      return { bottom: 80, left: 16 }
    }

    const rect = containerRef.current.getBoundingClientRect()
    return {
      bottom: window.innerHeight - rect.top + 8,
      left: rect.left,
    }
  }

  const isDisabled = Boolean(disabled || isSubmitting)

  return (
    <div
      ref={containerRef}
      data-command-input="true"
      className="shrink-0 border-t border-[var(--ide-border-subtle)] bg-[var(--ide-surface-1)] px-2 pb-1.5 pt-1 md:px-2.5"
    >
      {showPalette && (
        <CommandPalette
          filter={paletteFilter}
          onSelect={handleCommandSelect}
          onClose={() => {
            setShowPalette(false)
            setValue("")
          }}
          position={getPalettePosition()}
          runtimeCapabilities={runtimeCapabilities}
        />
      )}

      {(value.length > 0 || pendingImages.length > 0 || pendingMentions.length > 0) && (
        <div className="mb-1 flex items-center justify-end text-[10px] text-muted-foreground/50">
          <span>{value.length > 0 ? `${value.length} caracteres` : "anexos prontos"}</span>
        </div>
      )}

      {(pendingImages.length > 0 || pendingMentions.length > 0) && (
        <div className="mb-1 flex flex-wrap gap-1">
          {pendingMentions.map((path, index) => (
            <span
              key={`mention-${path}-${index}`}
              className="inline-flex items-center gap-1 rounded border border-terminal-blue/25 bg-terminal-blue/10 px-1.5 py-0.5 text-[10px] text-terminal-blue"
            >
              <AtSign className="h-3 w-3" />
              {fileLabel(path)}
              {onRemoveMention && (
                <button
                  type="button"
                  aria-label={`Remover mencao ${fileLabel(path)}`}
                  onClick={() => onRemoveMention(index)}
                  className="hover:text-terminal-fg"
                >
                  <X className="h-3 w-3" />
                </button>
              )}
            </span>
          ))}

          {pendingImages.map((path, index) => (
            <span
              key={`image-${path}-${index}`}
              className="inline-flex items-center gap-1 rounded border border-terminal-purple/25 bg-terminal-purple/10 px-1.5 py-0.5 text-[10px] text-terminal-purple"
            >
              <ImagePlus className="h-3 w-3" />
              {fileLabel(path)}
              {onRemoveImage && (
                <button
                  type="button"
                  aria-label={`Remover imagem ${fileLabel(path)}`}
                  onClick={() => onRemoveImage(index)}
                  className="hover:text-terminal-fg"
                >
                  <X className="h-3 w-3" />
                </button>
              )}
            </span>
          ))}
        </div>
      )}

      <div className="flex items-end gap-1.5 rounded border border-[var(--ide-border-subtle)] bg-[var(--terminal-bg)] px-2 py-1 transition-colors focus-within:border-terminal-blue/45">
        <div className="flex items-center gap-1 py-0.5">
          <span className="select-none text-xs font-semibold text-terminal-blue/90">{">"}</span>
        </div>

        <textarea
          ref={textareaRef}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Pergunte para a Alicia ou digite / para comandos"
          disabled={isDisabled}
          rows={1}
          className="min-h-[20px] max-h-[200px] flex-1 resize-none bg-transparent py-0.5 text-[12px] leading-normal text-terminal-fg outline-none placeholder:text-muted-foreground/45 disabled:opacity-50"
        />

        <div className="flex items-center gap-0.5 py-0.5">
          <button
            type="button"
            aria-label="Anexar arquivo"
            onClick={() => void onAttachMention?.()}
            className="rounded p-1 text-muted-foreground/45 transition-colors hover:bg-[var(--ide-hover)] hover:text-muted-foreground"
            title="Anexar arquivo"
            disabled={isDisabled}
          >
            <Paperclip className="w-4 h-4" />
          </button>

          <button
            type="button"
            aria-label="Anexar imagem"
            onClick={() => void onAttachImage?.()}
            className="rounded p-1 text-muted-foreground/45 transition-colors hover:bg-[var(--ide-hover)] hover:text-muted-foreground"
            title="Anexar imagem"
            disabled={isDisabled}
          >
            <ImagePlus className="w-4 h-4" />
          </button>

          <button
            type="button"
            aria-label="Mencionar arquivo ou simbolo"
            onClick={() => void onAttachMention?.()}
            className="rounded p-1 text-muted-foreground/45 transition-colors hover:bg-[var(--ide-hover)] hover:text-muted-foreground"
            title="Mencionar arquivo ou simbolo"
            disabled={isDisabled}
          >
            <AtSign className="w-4 h-4" />
          </button>

          <button
            type="button"
            aria-label="Enviar prompt"
            onClick={() => void handleSubmit()}
            disabled={isDisabled || (!value.trim() && pendingImages.length === 0 && pendingMentions.length === 0)}
            className="ml-0.5 rounded border border-terminal-blue/30 bg-terminal-blue/10 p-1 text-terminal-blue transition-colors hover:bg-terminal-blue/20 disabled:cursor-not-allowed disabled:opacity-30"
          >
            <ArrowUp className="w-4 h-4" />
          </button>
        </div>
      </div>

      <div className="mt-1 flex items-center justify-start px-0.5">
        <div className="flex items-center gap-2 overflow-x-auto text-[10px] text-muted-foreground/35">
          <span>
            <kbd className="px-1 py-0.5 rounded bg-background/50 border border-panel-border text-muted-foreground/50">Enter</kbd>
            {" enviar"}
          </span>
          <span>
            <kbd className="px-1 py-0.5 rounded bg-background/50 border border-panel-border text-muted-foreground/50">Shift+Enter</kbd>
            {" nova linha"}
          </span>
          <span>
            <kbd className="px-1 py-0.5 rounded bg-background/50 border border-panel-border text-muted-foreground/50">/</kbd>
            {" comandos"}
          </span>
        </div>
      </div>
    </div>
  )
}
