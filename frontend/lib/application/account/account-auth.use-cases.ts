import type { RuntimeClientPort } from "@/lib/application/ports/runtime-client.port"
import type {
  AccountLoginStartResponse,
  AccountLogoutResponse,
} from "@/lib/application/runtime-types"

export async function startChatgptAccountLogin(
  runtimeClient: RuntimeClientPort,
): Promise<AccountLoginStartResponse> {
  return runtimeClient.accountLoginStart({ type: "chatgpt" })
}

export async function loginWithApiKey(
  apiKey: string,
  runtimeClient: RuntimeClientPort,
): Promise<AccountLoginStartResponse> {
  return runtimeClient.accountLoginStart({ type: "apiKey", apiKey })
}

export async function logoutAccount(
  runtimeClient: RuntimeClientPort,
): Promise<AccountLogoutResponse> {
  return runtimeClient.accountLogout()
}
