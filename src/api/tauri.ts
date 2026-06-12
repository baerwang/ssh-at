import { invoke } from '@tauri-apps/api/core'

// Full version matching Rust backend HostEntry
export interface HostEntry {
  host: string
  hostname?: string
  user?: string
  port?: number
  identity_file?: string
  proxy_jump?: string
  proxy_command?: string
  forward_agent?: boolean
  strict_host_key_checking?: string
  server_alive_interval?: number
  server_alive_count_max?: number
  compression?: boolean
  connection_attempts?: number
  connect_timeout?: number
  local_forward?: string
  remote_forward?: string
  dynamic_forward?: string
  pubkey_accepted_key_types?: string
  host_key_algorithms?: string
  extra_options?: Record<string, string>
}

export interface SshConfig {
  hosts: HostEntry[]
  global_options?: Record<string, string>
}

export interface KeyInfo {
  path: string
  key_type: string
  fingerprint: string
  comment?: string
  size?: number
  created?: string
  is_encrypted: boolean
}

export interface BackupInfo {
  id: number
  timestamp: string
  file_path: string
  config_hash: string
  host_count: number
  size_bytes: number
}

// Config operations
export const loadSshConfig = (): Promise<SshConfig> =>
  invoke('load_ssh_config')

export const saveSshConfig = (config: SshConfig): Promise<void> =>
  invoke('save_ssh_config', { config })

export const serializeSshConfig = (config: SshConfig): Promise<string> =>
  invoke('serialize_ssh_config', { config })

export const parseSshConfig = (content: string): Promise<SshConfig> =>
  invoke('parse_ssh_config', { content })

export const addHost = (entry: HostEntry): Promise<void> =>
  invoke('add_host', { entry })

export const updateHost = (name: string, entry: HostEntry): Promise<void> =>
  invoke('update_host', { name, entry })

export const deleteHost = async (name: string): Promise<void> => {
  console.log('[tauri.ts deleteHost] 🔥 Invoking delete_host command with:', name)
  try {
    await invoke('delete_host', { name })
    console.log('[tauri.ts deleteHost] 🔥 Command succeeded')
  } catch (error) {
    console.error('[tauri.ts deleteHost] 🔥 Command failed with error:', error)
    throw error
  }
}

export const searchHosts = (query: string): Promise<HostEntry[]> =>
  invoke('search_hosts', { query })

// Key operations
export const scanSshKeys = (): Promise<KeyInfo[]> =>
  invoke('scan_ssh_keys')

export const getKeyFingerprint = (path: string): Promise<string> =>
  invoke('get_key_fingerprint', { path })

export const generateSshKey = (
  keyType: string,
  name: string,
  comment?: string,
  passphrase?: string,
  bits?: number
): Promise<void> =>
  invoke('generate_ssh_key', { keyType, name, comment, passphrase, bits })

export const deleteSshKey = (path: string): Promise<void> =>
  invoke('delete_ssh_key', { path })

export const readPublicKey = (privateKeyPath: string): Promise<string> =>
  invoke('read_public_key', { privateKeyPath })

// Backup operations
export const listBackups = (): Promise<BackupInfo[]> =>
  invoke('list_backups')

export const restoreBackup = (backupId: number): Promise<void> =>
  invoke('restore_backup', { backupId })

export const deleteBackup = async (backupId: number): Promise<void> => {
  console.log('[tauri.ts deleteBackup] 🔥 Invoking delete_backup command with:', backupId)
  try {
    await invoke('delete_backup', { backupId })
    console.log('[tauri.ts deleteBackup] 🔥 Command succeeded')
  } catch (error) {
    console.error('[tauri.ts deleteBackup] 🔥 Command failed with error:', error)
    throw error
  }
}

// System operations
export const openConfigDir = (): Promise<void> =>
  invoke('open_config_dir')

// Settings operations
export interface AppSettings {
  auto_backup: boolean
  backup_limit: number
  confirm_delete: boolean
}

export const loadSettings = (): Promise<AppSettings> =>
  invoke('load_settings')

export const saveSettings = (settings: AppSettings): Promise<void> =>
  invoke('save_settings', { settings })
