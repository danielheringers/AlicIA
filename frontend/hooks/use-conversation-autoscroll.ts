import { useCallback, useEffect, useRef, type RefObject } from "react"
import type { Message } from "@/lib/alicia-runtime-helpers"

interface UseConversationAutoscrollOptions {
  initializing: boolean
  messages: Message[]
  isThinking: boolean
  pendingApprovalsCount: number
  turnDiffTurnId: string | null
  turnPlanTurnId: string | null
}

interface UseConversationAutoscrollResult {
  scrollRef: RefObject<HTMLDivElement | null>
}

export function useConversationAutoscroll({
  initializing,
  messages,
  isThinking,
  pendingApprovalsCount,
  turnDiffTurnId,
  turnPlanTurnId,
}: UseConversationAutoscrollOptions): UseConversationAutoscrollResult {
  const scrollRef = useRef<HTMLDivElement>(null)
  const shouldAutoScrollRef = useRef(true)

  const isNearBottom = useCallback((container: HTMLDivElement) => {
    const thresholdPx = 96
    const remaining =
      container.scrollHeight - container.scrollTop - container.clientHeight
    return remaining <= thresholdPx
  }, [])

  const scrollConversationToBottom = useCallback((force = false) => {
    const container = scrollRef.current
    if (!container) {
      return
    }

    if (!force && !shouldAutoScrollRef.current) {
      return
    }

    container.scrollTop = container.scrollHeight
    shouldAutoScrollRef.current = true
  }, [])

  useEffect(() => {
    const container = scrollRef.current
    if (!container) {
      return
    }

    const syncAutoScroll = () => {
      shouldAutoScrollRef.current = isNearBottom(container)
    }

    scrollConversationToBottom(true)
    syncAutoScroll()
    container.addEventListener("scroll", syncAutoScroll, { passive: true })

    return () => {
      container.removeEventListener("scroll", syncAutoScroll)
    }
  }, [initializing, isNearBottom, scrollConversationToBottom])

  useEffect(() => {
    const frameId = window.requestAnimationFrame(() => {
      scrollConversationToBottom()
    })

    return () => {
      window.cancelAnimationFrame(frameId)
    }
  }, [
    messages,
    isThinking,
    pendingApprovalsCount,
    turnDiffTurnId,
    turnPlanTurnId,
    scrollConversationToBottom,
  ])

  return { scrollRef }
}