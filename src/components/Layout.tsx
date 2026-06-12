import { Box, Drawer, List, ListItem, ListItemButton, ListItemIcon, ListItemText, Typography } from '@mui/material'
import { Computer, VpnKey, Backup, Settings, Code, Terminal } from '@mui/icons-material'
import { Routes, Route, useNavigate, useLocation } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import HostsPage from '../pages/HostsPage'
import KeysPage from '../pages/KeysPage'
import BackupsPage from '../pages/BackupsPage'
import SettingsPage from '../pages/SettingsPage'
import ConfigEditorPage from '../pages/ConfigEditorPage'

const drawerWidth = 240

function Layout() {
  const navigate = useNavigate()
  const location = useLocation()
  const { t } = useTranslation()

  const menuItems = [
    { text: t('menu.hosts'), icon: <Computer />, path: '/' },
    { text: t('menu.sshKeys'), icon: <VpnKey />, path: '/keys' },
    { text: t('menu.configEditor'), icon: <Code />, path: '/editor' },
    { text: t('menu.backups'), icon: <Backup />, path: '/backups' },
    { text: t('menu.settings'), icon: <Settings />, path: '/settings' },
  ]

  return (
    <Box sx={{ display: 'flex', height: '100vh' }}>
      <Drawer
        variant="permanent"
        sx={{
          width: drawerWidth,
          flexShrink: 0,
          '& .MuiDrawer-paper': {
            width: drawerWidth,
            boxSizing: 'border-box',
          },
        }}
      >
        {/* Logo区域 */}
        <Box
          sx={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            mt: 2,
            py: 1,
            px: 2,
            gap: 0.5,
          }}
        >
          <Box
            sx={{
              width: 60,
              height: 60,
              borderRadius: '10px',
              background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              boxShadow: '0 4px 12px rgba(102, 126, 234, 0.3)',
            }}
          >
            <Terminal sx={{ fontSize: 36, color: 'white' }} />
          </Box>
          <Typography
            variant="subtitle2"
            sx={{
              fontWeight: 600,
              color: 'text.secondary',
              letterSpacing: 1,
            }}
          >
            SSH-AT
          </Typography>
        </Box>

        <Box sx={{ overflow: 'auto' }}>
          <List>
            {menuItems.map((item) => (
              <ListItem key={item.text} disablePadding>
                <ListItemButton
                  selected={location.pathname === item.path}
                  onClick={() => navigate(item.path)}
                >
                  <ListItemIcon>{item.icon}</ListItemIcon>
                  <ListItemText primary={item.text} />
                </ListItemButton>
              </ListItem>
            ))}
          </List>
        </Box>
      </Drawer>
      <Box component="main" sx={{ flexGrow: 1, p: 3, overflow: 'auto' }}>
        <Routes>
          <Route path="/" element={<HostsPage />} />
          <Route path="/keys" element={<KeysPage />} />
          <Route path="/editor" element={<ConfigEditorPage />} />
          <Route path="/backups" element={<BackupsPage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </Box>
    </Box>
  )
}

export default Layout
