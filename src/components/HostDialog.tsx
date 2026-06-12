import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  TextField,
  Grid,
  FormControlLabel,
  Switch,
  MenuItem,
  Box,
  Typography,
  Tabs,
  Tab,
  Divider,
  Autocomplete,
} from '@mui/material'
import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { HostEntry } from '../api/tauri'
import { scanSshKeys } from '../api/tauri'

interface HostDialogProps {
  open: boolean
  host?: HostEntry
  onClose: () => void
  onSave: (host: HostEntry) => void
}

interface TabPanelProps {
  children?: React.ReactNode
  index: number
  value: number
}

function TabPanel(props: TabPanelProps) {
  const { children, value, index, ...other } = props
  return (
    <div hidden={value !== index} {...other}>
      {value === index && <Box sx={{ py: 2 }}>{children}</Box>}
    </div>
  )
}

function HostDialog({ open, host, onClose, onSave }: HostDialogProps) {
  const [tabValue, setTabValue] = useState(0)
  const [proxyType, setProxyType] = useState<'none' | 'jump' | 'socks5' | 'custom'>('none')
  const [socks5Host, setSocks5Host] = useState('')
  const [socks5Port, setSocks5Port] = useState('1080')
  const { t } = useTranslation()

  // Fetch SSH keys for certificate dropdown
  const { data: sshKeys } = useQuery({
    queryKey: ['sshKeys'],
    queryFn: scanSshKeys,
    staleTime: 30000, // Cache for 30 seconds
  })

  const [formData, setFormData] = useState<HostEntry>({
    host: '',
    hostname: '',
    user: '',
    port: 22,
    identity_file: '',
    proxy_jump: '',
    proxy_command: '',
    forward_agent: undefined,
    strict_host_key_checking: undefined,
    server_alive_interval: undefined,
    server_alive_count_max: undefined,
    compression: undefined,
    connection_attempts: undefined,
    connect_timeout: undefined,
    local_forward: '',
    remote_forward: '',
    dynamic_forward: '',
    pubkey_accepted_key_types: '',
    host_key_algorithms: '',
    extra_options: {},
  })

  useEffect(() => {
    if (host) {
      setFormData({
        ...host,
        port: host.port ?? 22, // 默认22端口
      })
      // Detect proxy type from existing config
      if (host.proxy_jump) {
        setProxyType('jump')
      } else if (host.proxy_command?.includes('nc -x')) {
        setProxyType('socks5')
        // Extract SOCKS5 host and port from ProxyCommand
        const match = host.proxy_command.match(/nc -x ([^:]+):(\d+)/)
        if (match) {
          setSocks5Host(match[1])
          setSocks5Port(match[2])
        }
      } else if (host.proxy_command) {
        setProxyType('custom')
      } else {
        setProxyType('none')
      }
    } else {
      setFormData({
        host: '',
        hostname: '',
        user: '',
        port: 22,
        identity_file: '',
        proxy_jump: '',
        proxy_command: '',
        forward_agent: undefined,
        strict_host_key_checking: undefined,
        server_alive_interval: undefined,
        server_alive_count_max: undefined,
        compression: undefined,
        connection_attempts: undefined,
        connect_timeout: undefined,
        local_forward: '',
        remote_forward: '',
        dynamic_forward: '',
        pubkey_accepted_key_types: '',
        host_key_algorithms: '',
        extra_options: {},
      })
      setProxyType('none')
      setSocks5Host('')
      setSocks5Port('1080')
    }
  }, [host, open])

  const handleChange = (field: keyof HostEntry, value: any) => {
    if (field === 'host') {
      console.log('[HostDialog] Host field change - original value:', value)
      // 过滤空格字符
      value = value.replace(/\s+/g, '')
    }
    setFormData((prev) => ({ ...prev, [field]: value }))
  }

  const handleIdentityFileChange = (value: string | null) => {
    setFormData((prev) => ({
      ...prev,
      identity_file: value || '',
      // Auto-fill SSH algorithm configurations when certificate is selected
      pubkey_accepted_key_types: value ? '+ssh-rsa' : '',
      host_key_algorithms: value ? '+ssh-rsa' : '',
    }))
  }

  const handleProxyTypeChange = (type: 'none' | 'jump' | 'socks5' | 'custom') => {
    setProxyType(type)
    if (type === 'none') {
      handleChange('proxy_jump', '')
      handleChange('proxy_command', '')
    } else if (type === 'jump') {
      handleChange('proxy_command', '')
    } else if (type === 'socks5') {
      handleChange('proxy_jump', '')
      updateSocks5ProxyCommand()
    } else if (type === 'custom') {
      handleChange('proxy_jump', '')
    }
  }

  const updateSocks5ProxyCommand = () => {
    if (socks5Host && socks5Port) {
      const proxyCommand = `nc -x ${socks5Host}:${socks5Port} %h %p`
      handleChange('proxy_command', proxyCommand)
    }
  }

  useEffect(() => {
    if (proxyType === 'socks5') {
      updateSocks5ProxyCommand()
    }
  }, [socks5Host, socks5Port])

  const handleSubmit = async () => {
    console.log('[HostDialog] handleSubmit - formData.host:', formData.host)

    // Validate required fields
    const trimmedHost = formData.host.trim()
    const trimmedHostname = formData.hostname?.trim()
    const trimmedUser = formData.user?.trim()

    if (!trimmedHost) {
      alert(t('hostDialog.errorHostRequired') || 'Host name is required')
      return
    }

    if (!trimmedHostname) {
      alert(t('hostDialog.errorHostnameRequired') || 'HostName (IP/Domain) is required')
      return
    }

    if (!trimmedUser) {
      alert(t('hostDialog.errorUserRequired') || 'User is required')
      return
    }

    // 清理空字符串和默认值，转换为 null（而非 undefined），这样后端 Option<T> 会是 None
    const cleanedData: HostEntry = {
      host: trimmedHost,
      hostname: trimmedHostname,
      user: trimmedUser,
      port: formData.port === 22 ? undefined : formData.port, // 22 是默认端口，不保存
      identity_file: formData.identity_file?.trim() || undefined,
      proxy_jump: formData.proxy_jump?.trim() || undefined,
      proxy_command: formData.proxy_command?.trim() || undefined,
      forward_agent: formData.forward_agent, // 保持原值，如果是 undefined 就是 undefined
      strict_host_key_checking: formData.strict_host_key_checking?.trim() || undefined,
      server_alive_interval: formData.server_alive_interval === 0 ? undefined : formData.server_alive_interval,
      server_alive_count_max: formData.server_alive_count_max,
      compression: formData.compression,
      connection_attempts: formData.connection_attempts,
      connect_timeout: formData.connect_timeout,
      local_forward: formData.local_forward?.trim() || undefined,
      remote_forward: formData.remote_forward?.trim() || undefined,
      dynamic_forward: formData.dynamic_forward?.trim() || undefined,
      pubkey_accepted_key_types: formData.pubkey_accepted_key_types?.trim() || undefined,
      host_key_algorithms: formData.host_key_algorithms?.trim() || undefined,
      extra_options: formData.extra_options,
    }

    try {
      await onSave(cleanedData)
      // Only close dialog on success
      onClose()
    } catch (error) {
      // 解析后端返回的结构化错误码
      let errorMessage = String(error)
      try {
        const errorData = JSON.parse(String(error))
        if (errorData.code) {
          // 后端返回了结构化错误，根据 code 查翻译
          const translationKey = `validation.${errorData.code}`
          errorMessage = String(t(translationKey, errorData.params || {}))
        }
      } catch {
        // JSON 解析失败，直接显示原始错误
      }
      alert(`${t('validation.saveFailed')}: ${errorMessage}`)
    }
  }

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle>{host ? t('hostDialog.titleEdit') : t('hostDialog.titleAdd')}</DialogTitle>
      <DialogContent>
        <Tabs value={tabValue} onChange={(_, v) => setTabValue(v)} sx={{ borderBottom: 1, borderColor: 'divider' }}>
          <Tab label={t('hostDialog.tabBasic')} />
          <Tab label={t('hostDialog.tabProxy')} />
          <Tab label={t('hostDialog.tabAdvanced')} />
        </Tabs>

        <TabPanel value={tabValue} index={0}>
          <Grid container spacing={2}>
            <Grid item xs={12}>
              <TextField
                fullWidth
                required
                label={t('hostDialog.fieldHost')}
                value={formData.host}
                onChange={(e) => handleChange('host', e.target.value)}
                helperText={t('hostDialog.fieldHostHelper') + ' (空格会被自动移除)'}
                inputProps={{
                  autoCapitalize: 'none',
                  autoCorrect: 'off',
                  spellCheck: false
                }}
              />
            </Grid>

            <Grid item xs={12} sm={8}>
              <TextField
                fullWidth
                label={t('hostDialog.fieldHostname')}
                value={formData.hostname || ''}
                onChange={(e) => handleChange('hostname', e.target.value)}
                helperText={t('hostDialog.fieldHostnameHelper')}
              />
            </Grid>

            <Grid item xs={12} sm={4}>
              <TextField
                fullWidth
                type="number"
                label={t('hostDialog.fieldPort')}
                value={formData.port || 22}
                onChange={(e) => handleChange('port', parseInt(e.target.value))}
              />
            </Grid>

            <Grid item xs={12} sm={6}>
              <TextField
                fullWidth
                label={t('hostDialog.fieldUser')}
                value={formData.user || ''}
                onChange={(e) => handleChange('user', e.target.value)}
                inputProps={{
                  autoCapitalize: 'none',
                  autoCorrect: 'off',
                  spellCheck: false
                }}
              />
            </Grid>

            <Grid item xs={12} sm={6}>
              <Autocomplete
                fullWidth
                freeSolo
                options={sshKeys?.map((key) => key.path) || []}
                value={formData.identity_file || ''}
                onChange={(_, value) => handleIdentityFileChange(value)}
                onInputChange={(_, value) => handleIdentityFileChange(value)}
                renderInput={(params) => (
                  <TextField
                    {...params}
                    label={t('hostDialog.fieldIdentityFile')}
                    helperText={t('hostDialog.fieldIdentityFileHelper')}
                  />
                )}
                renderOption={(props, option) => {
                  const keyInfo = sshKeys?.find((key) => key.path === option)
                  return (
                    <li {...props} key={option}>
                      <Box>
                        <Typography variant="body2">{option}</Typography>
                        {keyInfo && (
                          <Typography variant="caption" color="text.secondary">
                            {keyInfo.key_type} {keyInfo.size ? `(${keyInfo.size} bits)` : ''}
                          </Typography>
                        )}
                      </Box>
                    </li>
                  )
                }}
              />
            </Grid>
          </Grid>
        </TabPanel>

        <TabPanel value={tabValue} index={1}>
          <Grid container spacing={2}>
            <Grid item xs={12}>
              <TextField
                fullWidth
                select
                label={t('hostDialog.proxyType')}
                value={proxyType}
                onChange={(e) => handleProxyTypeChange(e.target.value as any)}
              >
                <MenuItem value="none">{t('hostDialog.proxyTypeNone')}</MenuItem>
                <MenuItem value="jump">{t('hostDialog.proxyTypeJump')}</MenuItem>
                <MenuItem value="socks5">{t('hostDialog.proxyTypeSocks5')}</MenuItem>
                <MenuItem value="custom">{t('hostDialog.proxyTypeCustom')}</MenuItem>
              </TextField>
            </Grid>

            {proxyType === 'jump' && (
              <Grid item xs={12}>
                <TextField
                  fullWidth
                  label={t('hostDialog.fieldProxyJump')}
                  value={formData.proxy_jump || ''}
                  onChange={(e) => handleChange('proxy_jump', e.target.value)}
                  helperText={t('hostDialog.fieldProxyJumpHelper')}
                />
              </Grid>
            )}

            {proxyType === 'socks5' && (
              <>
                <Grid item xs={12}>
                  <Typography variant="body2" color="text.secondary" gutterBottom>
                    {t('hostDialog.socks5ConfigHint')}
                  </Typography>
                </Grid>
                <Grid item xs={12} sm={8}>
                  <TextField
                    fullWidth
                    label={t('hostDialog.fieldSocks5Host')}
                    value={socks5Host}
                    onChange={(e) => setSocks5Host(e.target.value)}
                    placeholder="127.0.0.1"
                  />
                </Grid>
                <Grid item xs={12} sm={4}>
                  <TextField
                    fullWidth
                    label={t('hostDialog.fieldSocks5Port')}
                    value={socks5Port}
                    onChange={(e) => setSocks5Port(e.target.value)}
                    placeholder="1080"
                  />
                </Grid>
                <Grid item xs={12}>
                  <Box sx={{ bgcolor: 'background.default', p: 2, borderRadius: 1 }}>
                    <Typography variant="caption" color="text.secondary">
                      {t('hostDialog.generatedProxyCommand')}:
                    </Typography>
                    <Typography variant="body2" fontFamily="monospace" sx={{ mt: 0.5 }}>
                      {formData.proxy_command || 'nc -x <host>:<port> %h %p'}
                    </Typography>
                  </Box>
                </Grid>
              </>
            )}

            {proxyType === 'custom' && (
              <Grid item xs={12}>
                <TextField
                  fullWidth
                  multiline
                  rows={3}
                  label={t('hostDialog.fieldProxyCommand')}
                  value={formData.proxy_command || ''}
                  onChange={(e) => handleChange('proxy_command', e.target.value)}
                  helperText={t('hostDialog.fieldProxyCommandHelper')}
                />
              </Grid>
            )}
          </Grid>
        </TabPanel>

        <TabPanel value={tabValue} index={2}>
          <Grid container spacing={2}>
            <Grid item xs={12} sm={6}>
              <TextField
                fullWidth
                select
                label={t('hostDialog.fieldStrictHostKeyChecking')}
                value={formData.strict_host_key_checking || 'ask'}
                onChange={(e) => handleChange('strict_host_key_checking', e.target.value)}
              >
                <MenuItem value="yes">{t('hostDialog.strictHostKeyCheckingYes')}</MenuItem>
                <MenuItem value="no">{t('hostDialog.strictHostKeyCheckingNo')}</MenuItem>
                <MenuItem value="ask">{t('hostDialog.strictHostKeyCheckingAsk')}</MenuItem>
              </TextField>
            </Grid>

            <Grid item xs={12} sm={6}>
              <TextField
                fullWidth
                type="number"
                label={t('hostDialog.fieldServerAliveInterval')}
                value={formData.server_alive_interval || 0}
                onChange={(e) => handleChange('server_alive_interval', parseInt(e.target.value))}
                helperText={t('hostDialog.fieldServerAliveIntervalHelper')}
              />
            </Grid>

            <Grid item xs={12}>
              <FormControlLabel
                control={
                  <Switch
                    checked={formData.forward_agent || false}
                    onChange={(e) => handleChange('forward_agent', e.target.checked)}
                  />
                }
                label={t('hostDialog.fieldForwardAgent')}
              />
            </Grid>

            <Grid item xs={12}>
              <Divider sx={{ my: 1 }} />
              <Typography variant="body2" color="text.secondary" gutterBottom>
                {t('hostDialog.additionalOptions')}
              </Typography>
            </Grid>

            <Grid item xs={12} sm={6}>
              <TextField
                fullWidth
                label={t('hostDialog.fieldLocalForward')}
                value={formData.local_forward || ''}
                onChange={(e) => handleChange('local_forward', e.target.value)}
                helperText={t('hostDialog.fieldLocalForwardHelper')}
              />
            </Grid>

            <Grid item xs={12} sm={6}>
              <TextField
                fullWidth
                label={t('hostDialog.fieldRemoteForward')}
                value={formData.remote_forward || ''}
                onChange={(e) => handleChange('remote_forward', e.target.value)}
                helperText={t('hostDialog.fieldRemoteForwardHelper')}
              />
            </Grid>

            <Grid item xs={12} sm={6}>
              <TextField
                fullWidth
                label={t('hostDialog.fieldDynamicForward')}
                value={formData.dynamic_forward || ''}
                onChange={(e) => handleChange('dynamic_forward', e.target.value)}
                helperText={t('hostDialog.fieldDynamicForwardHelper')}
              />
            </Grid>
          </Grid>
        </TabPanel>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>{t('hostDialog.cancel')}</Button>
        <Button
          onClick={handleSubmit}
          variant="contained"
          disabled={!formData.host}
        >
          {t('hostDialog.save')}
        </Button>
      </DialogActions>
    </Dialog>
  )
}

export default HostDialog
