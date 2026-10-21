/**
 * BOS Authentication Client SDK
 * Production TypeScript interface for interacting with the `bos.auth` module over HTTP/JSON-RPC.
 */

import type {
  User,
  Role,
  UserSession,
  SecurityAuditLog,
  RegisterUserParams,
  RegisterUserResult,
  AuthenticateParams,
  AuthenticateResult,
  VerifySessionParams,
  VerifySessionResult,
  RevokeSessionParams,
  RevokeSessionResult,
  TerminateAllSessionsParams,
  TerminateAllSessionsResult,
  AssignRoleParams,
  AssignRoleResult,
  RevokeRoleParams,
  RevokeRoleResult,
  ChangePasswordParams,
  ChangePasswordResult,
  LockUserParams,
  LockUserResult,
  UnlockUserParams,
  UnlockUserResult,
  EnrollMfaParams,
  EnrollMfaResult,
} from "./types";

export interface AuthClientConfig {
  baseUrl: string;
  tenantId?: string;
  companyId?: string;
  sessionToken?: string;
}

export class AuthClient {
  private baseUrl: string;
  private tenantId?: string;
  private companyId?: string;
  private sessionToken?: string;

  constructor(config: AuthClientConfig) {
    this.baseUrl = config.baseUrl.replace(/\/+$/, "");
    this.tenantId = config.tenantId;
    this.companyId = config.companyId;
    this.sessionToken = config.sessionToken;
  }

  public setSessionToken(token?: string): void {
    this.sessionToken = token;
  }

  public getSessionToken(): string | undefined {
    return this.sessionToken;
  }

  private async request<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      ...(options.headers as Record<string, string> || {}),
    };

    if (this.sessionToken) {
      headers["Authorization"] = `Bearer ${this.sessionToken}`;
    }
    if (this.tenantId) {
      headers["x-bos-tenant-id"] = this.tenantId;
    }
    if (this.companyId) {
      headers["x-bos-company-id"] = this.companyId;
    }

    const response = await fetch(`${this.baseUrl}${endpoint}`, {
      ...options,
      headers,
    });

    if (!response.ok) {
      const errorBody = await response.text();
      let message = `HTTP ${response.status}: ${response.statusText}`;
      try {
        const parsed = JSON.parse(errorBody);
        if (parsed.error) message = parsed.error;
      } catch {
        if (errorBody) message = errorBody;
      }
      throw new Error(message);
    }

    return response.json() as Promise<T>;
  }

  /**
   * Execute an AuthManager codeunit procedure via RPC
   */
  public async callProcedure<T>(method: string, params: Record<string, unknown> = {}): Promise<T> {
    const payload = {
      codeunit: "AuthManager",
      method,
      params,
    };

    return this.request<T>("/api/v1/rpc/execute", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  }

  // --------------------------------------------------------------------------
  // AUTHENTICATION & IDENTITY OPERATIONS
  // --------------------------------------------------------------------------

  public async register(params: RegisterUserParams): Promise<RegisterUserResult> {
    return this.callProcedure<RegisterUserResult>("RegisterUser", params as unknown as Record<string, unknown>);
  }

  public async login(params: AuthenticateParams): Promise<AuthenticateResult> {
    const res = await this.callProcedure<AuthenticateResult>("Authenticate", params as unknown as Record<string, unknown>);
    if (res.authenticated && res.token) {
      this.setSessionToken(res.token);
    }
    return res;
  }

  public async verifySession(params?: Partial<VerifySessionParams>): Promise<VerifySessionResult> {
    const token = params?.token || this.sessionToken;
    if (!token) {
      return { valid: false, reason: "NO_ACTIVE_TOKEN" };
    }
    return this.callProcedure<VerifySessionResult>("VerifySession", {
      token,
      ipAddress: params?.ipAddress,
    });
  }

  public async logout(token?: string): Promise<RevokeSessionResult> {
    const targetToken = token || this.sessionToken;
    if (!targetToken) {
      return { success: true };
    }
    const res = await this.callProcedure<RevokeSessionResult>("RevokeSession", { token: targetToken });
    if (this.sessionToken === targetToken) {
      this.setSessionToken(undefined);
    }
    return res;
  }

  public async terminateAllSessions(username: string): Promise<TerminateAllSessionsResult> {
    return this.callProcedure<TerminateAllSessionsResult>("TerminateAllSessions", { username });
  }

  public async changePassword(params: ChangePasswordParams): Promise<ChangePasswordResult> {
    return this.callProcedure<ChangePasswordResult>("ChangePassword", params as unknown as Record<string, unknown>);
  }

  public async lockUser(username: string, reason?: string): Promise<LockUserResult> {
    return this.callProcedure<LockUserResult>("LockUser", { username, reason });
  }

  public async unlockUser(username: string): Promise<UnlockUserResult> {
    return this.callProcedure<UnlockUserResult>("UnlockUser", { username });
  }

  public async assignRole(params: AssignRoleParams): Promise<AssignRoleResult> {
    return this.callProcedure<AssignRoleResult>("AssignRole", params as unknown as Record<string, unknown>);
  }

  public async revokeRole(params: RevokeRoleParams): Promise<RevokeRoleResult> {
    return this.callProcedure<RevokeRoleResult>("RevokeRole", params as unknown as Record<string, unknown>);
  }

  public async enrollMfa(params: EnrollMfaParams): Promise<EnrollMfaResult> {
    return this.callProcedure<EnrollMfaResult>("EnrollMfa", params as unknown as Record<string, unknown>);
  }

  // --------------------------------------------------------------------------
  // MODEL QUERY OPERATIONS (RESTful)
  // --------------------------------------------------------------------------

  public async getUser(username: string): Promise<User | null> {
    return this.request<User | null>(`/api/v1/entities/User/${encodeURIComponent(username)}`);
  }

  public async listUsers(query: Record<string, string | number> = {}): Promise<{ data: User[]; total: number }> {
    const params = new URLSearchParams(query as Record<string, string>);
    return this.request<{ data: User[]; total: number }>(`/api/v1/entities/User?${params.toString()}`);
  }

  public async listRoles(): Promise<{ data: Role[]; total: number }> {
    return this.request<{ data: Role[]; total: number }>("/api/v1/entities/Role");
  }

  public async listActiveSessions(username?: string): Promise<{ data: UserSession[]; total: number }> {
    const qs = username ? `?username=${encodeURIComponent(username)}&sessionState=Active` : "?sessionState=Active";
    return this.request<{ data: UserSession[]; total: number }>(`/api/v1/entities/UserSession${qs}`);
  }

  public async listAuditLogs(username?: string): Promise<{ data: SecurityAuditLog[]; total: number }> {
    const qs = username ? `?username=${encodeURIComponent(username)}` : "";
    return this.request<{ data: SecurityAuditLog[]; total: number }>(`/api/v1/entities/SecurityAuditLog${qs}`);
  }
}
