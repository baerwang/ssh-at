import {
  Box,
  Card,
  CardContent,
  Typography,
  Switch,
  FormControlLabel,
  Divider,
  TextField,
  Button,
  RadioGroup,
  Radio,
  FormControl,
  FormLabel,
} from '@mui/material'
import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useThemeMode } from '../contexts/ThemeContext'
import { openConfigDir, loadSettings, saveSettings, type AppSettings } from '../api/tauri'

function SettingsPage() {
  const [autoBackup, setAutoBackup] = useState(true)
  const [backupLimit, setBackupLimit] = useState(10)
  const [confirmDelete, setConfirmDelete] = useState(true)
  const { themeMode, setThemeMode } = useThemeMode()
  const { t, i18n } = useTranslation()

  // 加载设置
  useEffect(() => {
    loadSettings().then((settings) => {
      setAutoBackup(settings.auto_backup)
      setBackupLimit(settings.backup_limit)
      setConfirmDelete(settings.confirm_delete)
    }).catch((error) => {
      console.error('Failed to load settings:', error)
    })
  }, [])

  // 保存设置的通用函数
  const handleSaveSettings = async (updates: Partial<AppSettings>) => {
    const newSettings: AppSettings = {
      auto_backup: autoBackup,
      backup_limit: backupLimit,
      confirm_delete: confirmDelete,
      ...updates,
    }

    try {
      await saveSettings(newSettings)
    } catch (error) {
      console.error('Failed to save settings:', error)
      alert(`保存设置失败: ${error}`)
    }
  }

  const handleLanguageChange = (lng: string) => {
    i18n.changeLanguage(lng)
  }

  return (
    <Box>
      <Typography variant="h4" gutterBottom>
        {t('settings.title')}
      </Typography>

      <Card sx={{ mb: 2 }}>
        <CardContent>
          <Typography variant="h6" gutterBottom>
            {t('settings.appearance')}
          </Typography>

          <FormControl component="fieldset" sx={{ mb: 3 }}>
            <FormLabel component="legend">{t('settings.themeMode')}</FormLabel>
            <RadioGroup
              value={themeMode}
              onChange={(e) => setThemeMode(e.target.value as 'light' | 'dark' | 'system')}
            >
              <FormControlLabel value="system" control={<Radio />} label={t('settings.themeSystem')} />
              <FormControlLabel value="light" control={<Radio />} label={t('settings.themeLight')} />
              <FormControlLabel value="dark" control={<Radio />} label={t('settings.themeDark')} />
            </RadioGroup>
          </FormControl>

          <FormControl component="fieldset">
            <FormLabel component="legend">{t('settings.language')}</FormLabel>
            <RadioGroup
              value={i18n.language}
              onChange={(e) => handleLanguageChange(e.target.value)}
            >
              <FormControlLabel value="zh-CN" control={<Radio />} label={t('settings.languageChinese')} />
              <FormControlLabel value="en-US" control={<Radio />} label={t('settings.languageEnglish')} />
            </RadioGroup>
          </FormControl>
        </CardContent>
      </Card>

      <Card sx={{ mb: 2 }}>
        <CardContent>
          <Typography variant="h6" gutterBottom>
            {t('settings.backupSettings')}
          </Typography>

          <FormControlLabel
            control={
              <Switch
                checked={autoBackup}
                onChange={(e) => {
                  const newValue = e.target.checked
                  setAutoBackup(newValue)
                  handleSaveSettings({ auto_backup: newValue })
                }}
              />
            }
            label={t('settings.autoBackup')}
          />

          <Box sx={{ mt: 2 }}>
            <TextField
              type="number"
              label={t('settings.maxBackups')}
              value={backupLimit}
              onChange={(e) => {
                const newValue = parseInt(e.target.value)
                setBackupLimit(newValue)
              }}
              onBlur={() => {
                handleSaveSettings({ backup_limit: backupLimit })
              }}
              inputProps={{ min: 1, max: 50 }}
              helperText={t('settings.maxBackupsHelp')}
              sx={{ width: 250 }}
            />
          </Box>
        </CardContent>
      </Card>

      <Card sx={{ mb: 2 }}>
        <CardContent>
          <Typography variant="h6" gutterBottom>
            {t('settings.editorSettings')}
          </Typography>

          <FormControlLabel
            control={
              <Switch
                checked={confirmDelete}
                onChange={(e) => {
                  const newValue = e.target.checked
                  setConfirmDelete(newValue)
                  handleSaveSettings({ confirm_delete: newValue })
                }}
              />
            }
            label={t('settings.confirmDelete')}
          />
        </CardContent>
      </Card>

      <Card>
        <CardContent>
          <Typography variant="h6" gutterBottom>
            {t('settings.about')}
          </Typography>

          <Typography variant="body2" color="text.secondary" paragraph>
            {t('settings.aboutDescription')}
          </Typography>

          <Typography variant="body2" color="text.secondary" paragraph>
            {t('settings.version')}: 0.1.0
          </Typography>

          <Divider sx={{ my: 2 }} />

          <Typography variant="body2" color="text.secondary" paragraph>
            {t('settings.sshConfigLocation')}: ~/.ssh/config
          </Typography>

          <Typography variant="body2" color="text.secondary" paragraph>
            {t('settings.backupLocation')}: ~/.ssh-at/backups/
          </Typography>

          <Button variant="outlined" size="small" sx={{ mt: 1 }} onClick={async () => {
            try {
              await openConfigDir()
            } catch (error) {
              alert(`打开目录失败: ${error}`)
            }
          }}>
            {t('settings.openConfigDir')}
          </Button>
        </CardContent>
      </Card>
    </Box>
  )
}

export default SettingsPage
