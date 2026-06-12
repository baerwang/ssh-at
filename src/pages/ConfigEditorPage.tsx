import { Box, Button, Paper, Typography, Alert } from '@mui/material'
import { Save, Refresh } from '@mui/icons-material'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import Editor from '@monaco-editor/react'
import { useThemeMode } from '../contexts/ThemeContext'
import { loadSshConfig, serializeSshConfig, parseSshConfig, saveSshConfig } from '../api/tauri'

function ConfigEditorPage() {
  const [content, setContent] = useState('')
  const [hasChanges, setHasChanges] = useState(false)
  const [parseError, setParseError] = useState<string | null>(null)
  const queryClient = useQueryClient()
  const { mode } = useThemeMode()
  const { t } = useTranslation()

  const { data: config, isLoading, error } = useQuery({
    queryKey: ['sshConfig'],
    queryFn: loadSshConfig,
  })

  const serializeMutation = useMutation({
    mutationFn: serializeSshConfig,
    onSuccess: (text) => {
      setContent(text)
    },
  })

  const saveMutation = useMutation({
    mutationFn: async (text: string) => {
      const parsed = await parseSshConfig(text)
      await saveSshConfig(parsed)
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['sshConfig'] })
      setHasChanges(false)
      setParseError(null)
    },
    onError: (err: Error) => {
      setParseError(err.message)
    },
  })

  useEffect(() => {
    if (config) {
      serializeMutation.mutate(config)
    }
  }, [config])

  const handleSave = () => {
    saveMutation.mutate(content)
  }

  const handleRefresh = () => {
    queryClient.invalidateQueries({ queryKey: ['sshConfig'] })
    setHasChanges(false)
    setParseError(null)
  }

  const handleEditorChange = (value: string | undefined) => {
    setContent(value || '')
    setHasChanges(true)
    setParseError(null)
  }

  if (isLoading) {
    return (
      <Box sx={{ p: 3 }}>
        <Typography>{t('config.loading')}</Typography>
      </Box>
    )
  }

  if (error) {
    return (
      <Box sx={{ p: 3 }}>
        <Alert severity="error">
          {t('config.loadError')}: {(error as Error).message}
        </Alert>
      </Box>
    )
  }

  return (
    <Box>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 2 }}>
        <Typography variant="h4">{t('config.title')}</Typography>
        <Box>
          <Button
            startIcon={<Refresh />}
            onClick={handleRefresh}
            disabled={!hasChanges}
            sx={{ mr: 1 }}
          >
            {t('config.reset')}
          </Button>
          <Button
            variant="contained"
            startIcon={<Save />}
            onClick={handleSave}
            disabled={!hasChanges || saveMutation.isPending}
          >
            {saveMutation.isPending ? t('config.saving') : t('config.save')}
          </Button>
        </Box>
      </Box>

      {parseError && (
        <Alert severity="error" sx={{ mb: 2 }}>
          {t('config.parseError')}: {parseError}
        </Alert>
      )}

      <Alert severity="info" sx={{ mb: 2 }}>
        {t('config.infoMessage')}
      </Alert>

      <Paper sx={{ height: 'calc(100vh - 280px)', overflow: 'hidden' }}>
        <Editor
          height="100%"
          defaultLanguage="plaintext"
          theme={mode === 'dark' ? 'vs-dark' : 'light'}
          value={content}
          onChange={handleEditorChange}
          options={{
            minimap: { enabled: false },
            fontSize: 14,
            lineNumbers: 'on',
            wordWrap: 'on',
          }}
        />
      </Paper>
    </Box>
  )
}

export default ConfigEditorPage
