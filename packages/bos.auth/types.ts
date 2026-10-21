/**
 * BOS Enterprise User Management & Authentication Module (bos.auth)
 * TypeScript Type Definitions & Data Transfer Objects (DTOs)
 */

// ============================================================================
// 1. ENUMERATIONS
// ============================================================================

export type UserStatus = 
  | "Active" 
  | "Inactive" 
  | "Suspended" 
  | "LockedOut" 
  | "PendingVerification";

export type UserType = 
  | "Internal" 
  | "External" 
  | "ServiceAccount" 
  | "System";

export type SessionState = 
  | "Active" 
  | "Expired" 
  | "Revoked" 
  | "Terminated";

export type AuditAction = 
  | "LoginSuccess" 
  | "LoginFailed" 
  | "PasswordChanged" 
  | "RoleAssigned" 
  | "RoleRevoked" 
  | "AccountLocked" 
  | "AccountUnlocked" 
  | "SessionRevoked" 
  | "MfaEnrolled";

export type MfaMethod = 
  | "None" 
  | "TOTP" 
  | "EmailOTP" 
  | "SMSOTP" 
  | "WebAuthn";

// ============================================================================
// 2. DOMAIN ENTITY INTERFACES
// ============================================================================

export interface User {
  username: string;
  passwordHash: string;
  email: string;
  fullName?: string;
  phoneNumber?: string;
  userType: UserType;
  status: UserStatus;
  failedAttempts: number;
  lockoutUntil?: string | null;
  lastLoginAt?: string | null;
  passwordChangedAt?: string | null;
  isActive: boolean;
  activeSessionCount?: number;
  _id?: string;
  _tenant_id?: string;
  _created_at?: string;
  _updated_at?: string;
}

export interface Role {
  code: string;
  name: string;
  description?: string;
  isSystemRole: boolean;
  isActive: boolean;
  assignedUsersCount?: number;
  _id?: string;
  _tenant_id?: string;
  _created_at?: string;
  _updated_at?: string;
}

export interface UserRole {
  username: string;
  roleCode: string;
  assignedBy?: string;
  _id?: string;
  _tenant_id?: string;
  _created_at?: string;
}

export interface UserSession {
  token: string;
  username: string;
  ipAddress?: string;
  userAgent?: string;
  sessionState: SessionState;
  expiresAt: string;
  lastActivityAt?: string;
  _id?: string;
  _tenant_id?: string;
  _created_at?: string;
}

export interface UserSecuritySetting {
  username: string;
  mfaMethod: MfaMethod;
  mfaSecret?: string;
  mfaEnabled: boolean;
  maxConcurrentSessions: number;
  mustChangePassword: boolean;
  passwordExpiryDays: number;
  _id?: string;
  _tenant_id?: string;
  _created_at?: string;
  _updated_at?: string;
}

export interface SecurityAuditLog {
  id: string;
  username: string;
  action: AuditAction;
  ipAddress?: string;
  userAgent?: string;
  details?: string;
  isSuccess: boolean;
  _id?: string;
  _tenant_id?: string;
  _created_at?: string;
}

// ============================================================================
// 3. CODEUNIT PROCEDURE PARAMETERS & DTO RESULTS
// ============================================================================

export interface RegisterUserParams {
  username: string;
  email: string;
  password: string;
  fullName?: string;
  userType?: UserType;
}

export interface RegisterUserResult {
  success: boolean;
  username: string;
  email: string;
  userType: UserType;
}

export interface AuthenticateParams {
  username: string;
  password: string;
  ipAddress?: string;
  userAgent?: string;
}

export interface AuthenticateResult {
  authenticated: boolean;
  reason?: string;
  token?: string;
  username?: string;
  fullName?: string;
  email?: string;
  roles?: string[];
  expiresAt?: string;
  lockoutUntil?: string;
  attemptsRemaining?: number;
  mustChangePassword?: boolean;
  mfaRequired?: boolean;
}

export interface VerifySessionParams {
  token: string;
  ipAddress?: string;
}

export interface VerifySessionResult {
  valid: boolean;
  reason?: string;
  username?: string;
  roles?: string[];
  email?: string;
  expiresAt?: string;
}

export interface RevokeSessionParams {
  token: string;
}

export interface RevokeSessionResult {
  success: boolean;
  token?: string;
  reason?: string;
}

export interface TerminateAllSessionsParams {
  username: string;
}

export interface TerminateAllSessionsResult {
  success: boolean;
  terminatedCount: number;
}

export interface AssignRoleParams {
  username: string;
  roleCode: string;
  assignedBy?: string;
}

export interface AssignRoleResult {
  success: boolean;
  username?: string;
  roleCode?: string;
  message?: string;
}

export interface RevokeRoleParams {
  username: string;
  roleCode: string;
}

export interface RevokeRoleResult {
  success: boolean;
  username: string;
  roleCode: string;
}

export interface ChangePasswordParams {
  username: string;
  currentPassword: string;
  newPassword: string;
}

export interface ChangePasswordResult {
  success: boolean;
  username: string;
}

export interface LockUserParams {
  username: string;
  reason?: string;
}

export interface LockUserResult {
  success: boolean;
  username: string;
}

export interface UnlockUserParams {
  username: string;
}

export interface UnlockUserResult {
  success: boolean;
  username: string;
}

export interface EnrollMfaParams {
  username: string;
  method?: MfaMethod;
  secret: string;
}

export interface EnrollMfaResult {
  success: boolean;
  username: string;
  mfaMethod: MfaMethod;
}
