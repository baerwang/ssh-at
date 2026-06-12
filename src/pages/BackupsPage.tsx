import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import {
  Box,
  Button,
  Card,
  CardContent,
  IconButton,
  Typography,
  Chip,
  Alert,
} from '@mui/material'
import { Restore, Delete, Refresh } from '@mui/icons-material'
import { useTranslation } from 'react-i18next'
import { listBackups, restoreBackup, deleteBackup, loadSettings, type BackupInfo } from '../api/tauri'
import { ask } from '@tauri-apps/plugin-dialog'

function BackupsPage() {
  console.log('[BackupsPage] 🟢 Component mounted/re-rendered')
  const queryClient = useQueryClient()
  const { t } = useTranslation()

  const { data: backups, isLoading, refetch } = useQuery({
    queryKey: ['backups'],
    queryFn: listBackups,
    enabled: true, // Auto-load backups on page mount
  })

  const { data: settings } = useQuery({
    queryKey: ['settings'],
    queryFn: loadSettings,
  })

  const restoreMutation = useMutation({
    mutationFn: restoreBackup,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['backups'] })
      queryClient.invalidateQueries({ queryKey: ['sshConfig'] })
    },
  })

  const deleteMutation = useMutation({
    mutationFn: async (backupId: number) => {
      console.log('[BackupsPage deleteMutation] 🔥 mutationFn called with:', backupId)
      try {
        await deleteBackup(backupId)
        console.log('[BackupsPage deleteMutation] 🔥 deleteBackup succeeded')
      } catch (err) {
        console.error('[BackupsPage deleteMutation] 🔥 deleteBackup threw error:', err)
        throw err
      }
    },
    onSuccess: () => {
      console.log('[BackupsPage deleteMutation] 🔥 onSuccess called')
      queryClient.invalidateQueries({ queryKey: ['backups'] })
    },
    onError: (error) => {
      console.error('[BackupsPage deleteMutation] 🔥 onError called:', error)
      alert(`删除失败: ${error}`)
    },
  })

  const handleRestore = async (backupId: number) => {
    try {
      // 读取 settings，检查是否需要确认
      const settings = await loadSettings()

      if (settings.confirm_delete) {
        // 恢复操作也需要确认，因为会覆盖当前配置
        const confirmed = await ask(`确定要恢复此备份吗？这将覆盖当前的 SSH 配置。`, {
          title: '确认恢复',
          kind: 'warning',
        })

        if (!confirmed) {
          console.log('[BackupsPage] Restore cancelled by user')
          return
        }
      }

      await restoreMutation.mutateAsync(backupId)
    } catch (error) {
      console.error('[BackupsPage] Restore failed:', error)
      alert(`恢复失败: ${error}`)
    }
  }

  const handleDelete = async (backupId: number) => {
    console.log('[BackupsPage] handleDelete called with:', backupId)
    try {
      // 读取 settings，检查是否需要确认
      const settings = await loadSettings()

      if (settings.confirm_delete) {
        // 使用 Tauri dialog API 显示确认对话框
        const confirmed = await ask(`确定要删除这个备份吗？`, {
          title: '确认删除',
          kind: 'warning',
        })

        if (!confirmed) {
          console.log('[BackupsPage] Delete cancelled by user')
          return
        }
      }

      console.log('[BackupsPage] Calling deleteMutation.mutateAsync...')
      await deleteMutation.mutateAsync(backupId)
      console.log('[BackupsPage] Delete succeeded')
    } catch (error) {
      console.error('[BackupsPage] Delete failed in handleDelete:', error)
      alert(`删除失败: ${error}`)
    }
  }

  const formatBytes = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  }

  const formatTimestamp = (timestamp: string) => {
    // Format: YYYYMMDD_HHMMSS
    const year = timestamp.substring(0, 4)
    const month = timestamp.substring(4, 6)
    const day = timestamp.substring(6, 8)
    const hour = timestamp.substring(9, 11)
    const minute = timestamp.substring(11, 13)
    const second = timestamp.substring(13, 15)
    return `${year}-${month}-${day} ${hour}:${minute}:${second}`
  }

  return (
    <Box>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 3 }}>
        <Typography variant="h4">{t('backups.title')}</Typography>
        <Button startIcon={<Refresh />} onClick={() => refetch()}>
          {t('backups.refresh')}
        </Button>
      </Box>

      <Alert severity="info" sx={{ mb: 3 }}>
        {t('backups.infoMessage', { limit: settings?.backup_limit || 10 })}
      </Alert>

      {isLoading ? (
        <Typography>{t('backups.loading')}</Typography>
      ) : backups && backups.length > 0 ? (
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
          {backups.map((backup: BackupInfo) => (
            <Card key={backup.id}>
              <CardContent>
                <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'start' }}>
                  <Box sx={{ flex: 1 }}>
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
                      <Typography variant="h6">
                        {formatTimestamp(backup.timestamp)}
                      </Typography>
                      <Chip
                        label={`${backup.host_count} ${t('backups.hosts')}`}
                        size="small"
                        color="primary"
                      />
                      <Chip
                        label={formatBytes(backup.size_bytes)}
                        size="small"
                        variant="outlined"
                      />
                    </Box>

                    <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>
                      {backup.file_path}
                    </Typography>

                    <Typography
                      variant="caption"
                      color="text.secondary"
                      fontFamily="monospace"
                      sx={{ wordBreak: 'break-all' }}
                    >
                      {t('backups.hash')}: {backup.config_hash}
                    </Typography>
                  </Box>

                  <Box sx={{ display: 'flex', gap: 1 }}>
                    <Button
                      variant="outlined"
                      size="small"
                      startIcon={<Restore />}
                      onClick={() => handleRestore(backup.id)}
                    >
                      {t('backups.restore')}
                    </Button>
                    <IconButton
                      size="small"
                      color="error"
                      onClick={() => handleDelete(backup.id)}
                    >
                      <Delete />
                    </IconButton>
                  </Box>
                </Box>
              </CardContent>
            </Card>
          ))}
        </Box>
      ) : (
        <Card>
          <CardContent>
            <Typography color="text.secondary" align="center">
              {t('backups.noBackupsFound')}
            </Typography>
          </CardContent>
        </Card>
      )}
    </Box>
  )
}

export default BackupsPage
