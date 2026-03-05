import {
  useEffect,
  useRef,
  useState,
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
} from "react"
import type { Message } from "@/lib/alicia-runtime-helpers"

interface UseReviewChannelStateOptions {
  activeReviewSessionKey: string
  isThinking: boolean
  reviewRoutingRef: MutableRefObject<boolean>
}

interface UseReviewChannelStateResult {
  reviewMessages: Message[]
  setReviewMessages: Dispatch<SetStateAction<Message[]>>
  isReviewComplete: boolean
}

export function useReviewChannelState({
  activeReviewSessionKey,
  isThinking,
  reviewRoutingRef,
}: UseReviewChannelStateOptions): UseReviewChannelStateResult {
  const [reviewMessages, setReviewMessages] = useState<Message[]>([])
  const [isReviewComplete, setIsReviewComplete] = useState(false)

  const reviewMessagesBySessionRef = useRef<Map<string, Message[]>>(new Map())
  const activeReviewSessionKeyRef = useRef<string | null>(null)
  const wasReviewThinkingRef = useRef(false)

  useEffect(() => {
    if (activeReviewSessionKeyRef.current === null) {
      activeReviewSessionKeyRef.current = activeReviewSessionKey
      const initialMessages =
        reviewMessagesBySessionRef.current.get(activeReviewSessionKey) ?? []
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setReviewMessages(initialMessages)
      return
    }

    if (activeReviewSessionKeyRef.current === activeReviewSessionKey) {
      return
    }

    const previousKey = activeReviewSessionKeyRef.current
    if (previousKey) {
      reviewMessagesBySessionRef.current.set(previousKey, reviewMessages)
    }

    activeReviewSessionKeyRef.current = activeReviewSessionKey
    const nextMessages =
      reviewMessagesBySessionRef.current.get(activeReviewSessionKey) ?? []
    setReviewMessages(nextMessages)
  }, [activeReviewSessionKey, reviewMessages])

  useEffect(() => {
    reviewMessagesBySessionRef.current.set(activeReviewSessionKey, reviewMessages)
  }, [activeReviewSessionKey, reviewMessages])

  useEffect(() => {
    const nowThinking = isThinking && reviewRoutingRef.current
    if (nowThinking) {
      wasReviewThinkingRef.current = true
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setIsReviewComplete(false)
    } else if (wasReviewThinkingRef.current && !isThinking) {
      wasReviewThinkingRef.current = false
      setIsReviewComplete(true)
    }
  }, [isThinking, reviewRoutingRef])

  return {
    reviewMessages,
    setReviewMessages,
    isReviewComplete,
  }
}