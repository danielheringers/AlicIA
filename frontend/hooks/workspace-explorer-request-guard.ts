type ExplorerRequestToken = {
  key: string
  requestId: number
  workspaceEpoch: number
}

export interface WorkspaceExplorerRequestGuard {
  begin: (key: string) => ExplorerRequestToken
  isCurrent: (token: ExplorerRequestToken) => boolean
  invalidateAll: () => void
}

export function createWorkspaceExplorerRequestGuard(): WorkspaceExplorerRequestGuard {
  let workspaceEpoch = 0
  const latestRequestByKey = new Map<string, number>()

  const begin = (key: string): ExplorerRequestToken => {
    const requestId = (latestRequestByKey.get(key) ?? 0) + 1
    latestRequestByKey.set(key, requestId)
    return {
      key,
      requestId,
      workspaceEpoch,
    }
  }

  const isCurrent = ({ key, requestId, workspaceEpoch: tokenEpoch }: ExplorerRequestToken) => {
    if (tokenEpoch !== workspaceEpoch) {
      return false
    }
    return latestRequestByKey.get(key) === requestId
  }

  const invalidateAll = () => {
    workspaceEpoch += 1
    latestRequestByKey.clear()
  }

  return {
    begin,
    isCurrent,
    invalidateAll,
  }
}
