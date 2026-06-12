import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import {
  Box,
  Button,
  IconButton,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TextField,
  Typography,
  Chip,
} from '@mui/material'
import { Add, Edit, Delete, Search, Refresh } from '@mui/icons-material'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { loadSshConfig, deleteHost, addHost, updateHost, loadSettings, type HostEntry } from '../api/tauri'
import { ask } from '@tauri-apps/plugin-dialog'
import HostDialog from '../components/HostDialog'

function HostsPage() {
  console.log('[HostsPage] 🟢 Component mounted/re-rendered')
  const [searchQuery, setSearchQuery] = useState('')
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editingHost, setEditingHost] = useState<HostEntry | undefined>()
  const queryClient = useQueryClient()
  const { t } = useTranslation()

  const { data: config, isLoading, refetch } = useQuery({
    queryKey: ['sshConfig'],
    queryFn: loadSshConfig,
    enabled: true, // Re-enabled after fixing stack overflow
  })

  // Temporarily disable auto-fetch to debug
  // useEffect(() => {
  //   refetch()
  // }, [refetch])

  const addMutation = useMutation({
    mutationFn: addHost,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['sshConfig'] })
      queryClient.invalidateQueries({ queryKey: ['backups'] })
    },
  })

  const updateMutation = useMutation({
    mutationFn: ({ name, entry }: { name: string; entry: HostEntry }) =>
      updateHost(name, entry),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['sshConfig'] })
      queryClient.invalidateQueries({ queryKey: ['backups'] })
    },
  })

  const deleteMutation = useMutation({
    mutationFn: async (hostName: string) => {
      console.log('[deleteMutation] 🔥 mutationFn called with:', hostName)
      try {
        const result = await deleteHost(hostName)
        console.log('[deleteMutation] 🔥 deleteHost returned:', result)
        return result
      } catch (err) {
        console.error('[deleteMutation] 🔥 deleteHost threw error:', err)
        throw err
      }
    },
    onSuccess: () => {
      console.log('[deleteMutation] 🔥 onSuccess called')
      queryClient.invalidateQueries({ queryKey: ['sshConfig'] })
      queryClient.invalidateQueries({ queryKey: ['backups'] })
    },
    onError: (error) => {
      console.error('[deleteMutation] 🔥 onError called:', error)
      alert(`删除失败: ${error}`)
    },
  })

  const filteredHosts = config?.hosts.filter((host) => {
    const query = searchQuery.toLowerCase()
    return (
      host.host.toLowerCase().includes(query) ||
      host.hostname?.toLowerCase().includes(query) ||
      host.user?.toLowerCase().includes(query)
    )
  }).filter(host => host && host.host) // 过滤掉无效的 host 对象

  // 调试输出
  if (searchQuery) {
    console.log('[HostsPage] Search query:', searchQuery)
    console.log('[HostsPage] Filtered hosts count:', filteredHosts?.length)
    console.log('[HostsPage] Filtered hosts:', filteredHosts)
  }

  const handleDelete = async (hostName: string) => {
    console.log('[HostsPage] handleDelete called with:', hostName)

    try {
      // 读取 settings，检查是否需要确认
      const settings = await loadSettings()

      if (settings.confirm_delete) {
        // 使用 Tauri dialog API 显示确认对话框
        const confirmed = await ask(`确定要删除主机 "${hostName}" 吗？`, {
          title: '确认删除',
          kind: 'warning',
        })

        if (!confirmed) {
          console.log('[HostsPage] Delete cancelled by user')
          return
        }
      }

      console.log('[HostsPage] Calling deleteMutation.mutateAsync...')
      await deleteMutation.mutateAsync(hostName)
      console.log('[HostsPage] Delete succeeded')
    } catch (error) {
      console.error('[HostsPage] Delete failed in handleDelete:', error)
      alert(`删除失败: ${error}`)
    }
  }

  const handleAdd = () => {
    setEditingHost(undefined)
    setDialogOpen(true)
  }

  const handleEdit = (host: HostEntry) => {
    setEditingHost(host)
    setDialogOpen(true)
  }

  const handleSave = async (host: HostEntry) => {
    console.log('[HostsPage] handleSave - received host.host:', host.host)
    if (editingHost) {
      await updateMutation.mutateAsync({ name: editingHost.host, entry: host })
    } else {
      await addMutation.mutateAsync(host)
    }
  }

  return (
    <Box sx={{ p: 3 }}>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 4 }}>
        <Typography variant="h4" fontWeight={600}>
          {t('hosts.title')}
        </Typography>
        <Box sx={{ display: 'flex', gap: 1.5 }}>
          <Button startIcon={<Refresh />} onClick={() => refetch()}>
            {t('hosts.refresh')}
          </Button>
          <Button variant="contained" startIcon={<Add />} onClick={handleAdd}>
            {t('hosts.addHost')}
          </Button>
        </Box>
      </Box>

      <HostDialog
        open={dialogOpen}
        host={editingHost}
        onClose={() => setDialogOpen(false)}
        onSave={handleSave}
      />

      <Paper sx={{ mb: 3, p: 2.5 }}>
        <TextField
          fullWidth
          placeholder={t('hosts.searchPlaceholder')}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          InputProps={{
            startAdornment: <Search sx={{ mr: 1, color: 'text.secondary' }} />,
          }}
          inputProps={{
            autoCapitalize: 'none',
            autoCorrect: 'off',
            spellCheck: false
          }}
          sx={{
            '& .MuiOutlinedInput-root': {
              '& fieldset': { border: 'none' },
            },
          }}
        />
      </Paper>

      <TableContainer component={Paper} sx={{ overflow: 'hidden' }}>
        <Table>
          <TableHead>
            <TableRow sx={{ backgroundColor: 'rgba(0, 0, 0, 0.2)' }}>
              <TableCell sx={{ fontWeight: 600, py: 2 }}>{t('hosts.tableHost')}</TableCell>
              <TableCell sx={{ fontWeight: 600, py: 2 }}>{t('hosts.tableHostname')}</TableCell>
              <TableCell sx={{ fontWeight: 600, py: 2 }}>{t('hosts.tableUser')}</TableCell>
              <TableCell sx={{ fontWeight: 600, py: 2 }}>{t('hosts.tablePort')}</TableCell>
              <TableCell sx={{ fontWeight: 600, py: 2 }}>{t('hosts.tableProxy')}</TableCell>
              <TableCell align="right" sx={{ fontWeight: 600, py: 2 }}>{t('hosts.tableActions')}</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {isLoading ? (
              <TableRow>
                <TableCell colSpan={6} align="center">
                  {t('hosts.loading')}
                </TableCell>
              </TableRow>
            ) : filteredHosts && filteredHosts.length > 0 ? (
              filteredHosts.map((host: HostEntry, index: number) => (
                <TableRow
                  key={`${host.host}-${index}`}
                  hover
                  sx={{ '&:last-child td': { border: 0 } }}
                >
                  <TableCell sx={{ py: 2 }}>
                    <Typography variant="body2" fontWeight={600}>
                      {host.host}
                    </Typography>
                  </TableCell>
                  <TableCell sx={{ py: 2 }}>{host.hostname || '-'}</TableCell>
                  <TableCell sx={{ py: 2 }}>{host.user || '-'}</TableCell>
                  <TableCell sx={{ py: 2 }}>{host.port || 22}</TableCell>
                  <TableCell sx={{ py: 2 }}>
                    {host.proxy_jump && (
                      <Chip label={t('hosts.proxyJump')} size="small" color="primary" sx={{ mr: 0.5 }} />
                    )}
                    {host.proxy_command && (
                      <Chip label={t('hosts.proxyCommand')} size="small" color="secondary" />
                    )}
                  </TableCell>
                  <TableCell align="right" sx={{ py: 2 }}>
                    <IconButton
                      size="small"
                      color="primary"
                      onClick={() => handleEdit(host)}
                      sx={{ mr: 0.5 }}
                    >
                      <Edit fontSize="small" />
                    </IconButton>
                    <IconButton
                      size="small"
                      color="error"
                      onClick={() => handleDelete(host.host)}
                    >
                      <Delete fontSize="small" />
                    </IconButton>
                  </TableCell>
                </TableRow>
              ))
            ) : (
              <TableRow>
                <TableCell colSpan={6} align="center">
                  <Typography color="text.secondary">
                    {searchQuery ? t('hosts.noHostsFound') : t('hosts.noHostsConfigured')}
                  </Typography>
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </TableContainer>
    </Box>
  )
}

export default HostsPage
