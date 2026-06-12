import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import {
  Box,
  Button,
  Card,
  CardContent,
  Grid,
  IconButton,
  Typography,
  Chip,
  Tooltip,
} from '@mui/material'
import { Add, Refresh, Fingerprint, Lock, LockOpen, Delete } from '@mui/icons-material'
import { scanSshKeys, generateSshKey, deleteSshKey, readPublicKey, loadSettings, type KeyInfo } from '../api/tauri'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import GenerateKeyDialog from '../components/GenerateKeyDialog'
import { ask } from '@tauri-apps/plugin-dialog'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'

function KeysPage() {
  const [dialogOpen, setDialogOpen] = useState(false)
  const queryClient = useQueryClient()
  const { t } = useTranslation()

  const { data: keys, isLoading, refetch } = useQuery({
    queryKey: ['sshKeys'],
    queryFn: scanSshKeys,
    enabled: true, // Re-enabled after fixing stack overflow
  })

  const generateMutation = useMutation({
    mutationFn: (params: { keyType: string; name: string; comment?: string; passphrase?: string; bits?: number }) =>
      generateSshKey(params.keyType, params.name, params.comment, params.passphrase, params.bits),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['sshKeys'] })
      setDialogOpen(false)
      alert(t('keys.generateSuccess'))
    },
    onError: (error: any) => {
      const errorMsg = error?.message || error?.toString() || 'Unknown error'
      alert(t('keys.generateFailed') + ': ' + errorMsg)
    },
  })

  const deleteMutation = useMutation({
    mutationFn: (path: string) => deleteSshKey(path),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['sshKeys'] })
    },
    onError: (error: any) => {
      const errorMsg = error?.message || error?.toString() || 'Unknown error'
      alert(`${t('keys.deleteFailed')}: ${errorMsg}`)
    },
  })

  const handleDelete = async (key: KeyInfo) => {
    try {
      const settings = await loadSettings()

      if (settings.confirm_delete) {
        const confirmed = await ask(t('keys.deleteConfirm', { keyName: key.path }), {
          title: t('keys.deleteConfirmTitle'),
          kind: 'warning',
        })

        if (!confirmed) {
          return
        }
      }

      await deleteMutation.mutateAsync(key.path)
    } catch (error) {
      console.error('[KeysPage] Delete failed:', error)
    }
  }

  const handleCopyPublicKey = async (key: KeyInfo) => {
    try {
      const publicKeyContent = await readPublicKey(key.path)
      await writeText(publicKeyContent)
      // TODO: 可以添加成功提示 snackbar
    } catch (error) {
      console.error('[KeysPage] Copy public key failed:', error)
      const errorMsg = error instanceof Error ? error.message : String(error)
      alert(`${t('keys.copyFailed')}: ${errorMsg}`)
    }
  }

  const handleGenerate = async (keyType: string, name: string, comment?: string, passphrase?: string, bits?: number) => {
    await generateMutation.mutateAsync({ keyType, name, comment, passphrase, bits })
  }

  const getKeyTypeColor = (keyType: string) => {
    switch (keyType) {
      case 'Ed25519':
        return 'success'
      case 'RSA':
        return 'primary'
      case 'ECDSA':
        return 'info'
      default:
        return 'default'
    }
  }

  return (
    <Box>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 3 }}>
        <Typography variant="h4">{t('keys.title')}</Typography>
        <Box>
          <Button
            startIcon={<Refresh />}
            onClick={() => refetch()}
            sx={{ mr: 1 }}
          >
            {t('keys.refresh')}
          </Button>
          <Button
            variant="contained"
            startIcon={<Add />}
            onClick={() => setDialogOpen(true)}
          >
            {t('keys.generateKey')}
          </Button>
        </Box>
      </Box>

      <GenerateKeyDialog
        open={dialogOpen}
        onClose={() => setDialogOpen(false)}
        onGenerate={handleGenerate}
      />

      {isLoading ? (
        <Typography>{t('keys.loading')}</Typography>
      ) : keys && keys.length > 0 ? (
        <Grid container spacing={2}>
          {keys.map((key: KeyInfo) => (
            <Grid item xs={12} key={key.path}>
              <Card>
                <CardContent>
                  <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'start' }}>
                    <Box sx={{ flex: 1 }}>
                      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
                        <Typography variant="h6" component="div">
                          {key.path.split('/').pop()}
                        </Typography>
                        <Chip
                          label={key.key_type}
                          size="small"
                          color={getKeyTypeColor(key.key_type)}
                        />
                        {key.is_encrypted ? (
                          <Chip
                            icon={<Lock />}
                            label={t('keys.encrypted')}
                            size="small"
                            color="warning"
                          />
                        ) : (
                          <Chip
                            icon={<LockOpen />}
                            label={t('keys.unencrypted')}
                            size="small"
                          />
                        )}
                        {key.size && (
                          <Chip label={`${key.size} ${t('keys.bits')}`} size="small" variant="outlined" />
                        )}
                      </Box>

                      <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>
                        {key.path}
                      </Typography>

                      {key.comment && (
                        <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>
                          {t('keys.comment')}: {key.comment}
                        </Typography>
                      )}

                      {key.fingerprint && (
                        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                          <Fingerprint fontSize="small" color="action" />
                          <Typography
                            variant="body2"
                            fontFamily="monospace"
                            color="text.secondary"
                          >
                            {key.fingerprint}
                          </Typography>
                        </Box>
                      )}

                      {key.created && (
                        <Typography variant="caption" color="text.secondary" sx={{ mt: 1, display: 'block' }}>
                          {t('keys.created')}: {new Date(key.created).toLocaleString()}
                        </Typography>
                      )}
                    </Box>

                    <Box>
                      <Tooltip title={t('keys.copyToClipboard')}>
                        <IconButton size="small" onClick={() => handleCopyPublicKey(key)}>
                          <Fingerprint />
                        </IconButton>
                      </Tooltip>
                      <Tooltip title={t('common.delete')}>
                        <IconButton
                          size="small"
                          onClick={() => handleDelete(key)}
                          color="error"
                        >
                          <Delete />
                        </IconButton>
                      </Tooltip>
                    </Box>
                  </Box>
                </CardContent>
              </Card>
            </Grid>
          ))}
        </Grid>
      ) : (
        <Card>
          <CardContent>
            <Typography color="text.secondary" align="center">
              {t('keys.noKeysFound')}
            </Typography>
          </CardContent>
        </Card>
      )}
    </Box>
  )
}

export default KeysPage
