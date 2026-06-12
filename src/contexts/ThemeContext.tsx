import { createContext, useContext, useState, useEffect, ReactNode } from 'react'
import { ThemeProvider, createTheme } from '@mui/material/styles'
import { PaletteMode } from '@mui/material'

type ThemeMode = 'light' | 'dark' | 'system'

interface ThemeContextType {
  mode: PaletteMode
  themeMode: ThemeMode
  setThemeMode: (mode: ThemeMode) => void
  toggleTheme: () => void
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined)

export const useThemeMode = () => {
  const context = useContext(ThemeContext)
  if (!context) {
    throw new Error('useThemeMode must be used within ThemeContextProvider')
  }
  return context
}

interface ThemeContextProviderProps {
  children: ReactNode
}

export const ThemeContextProvider = ({ children }: ThemeContextProviderProps) => {
  const [themeMode, setThemeMode] = useState<ThemeMode>(() => {
    const saved = localStorage.getItem('themeMode')
    return (saved as ThemeMode) || 'system'
  })

  const [mode, setMode] = useState<PaletteMode>(() => {
    if (themeMode === 'system') {
      return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
    }
    return themeMode as PaletteMode
  })

  useEffect(() => {
    localStorage.setItem('themeMode', themeMode)

    if (themeMode === 'system') {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
      const updateMode = (e: MediaQueryListEvent) => {
        setMode(e.matches ? 'dark' : 'light')
      }

      setMode(mediaQuery.matches ? 'dark' : 'light')
      mediaQuery.addEventListener('change', updateMode)

      return () => mediaQuery.removeEventListener('change', updateMode)
    } else {
      setMode(themeMode as PaletteMode)
    }
  }, [themeMode])

  const toggleTheme = () => {
    if (themeMode === 'system') {
      setThemeMode('light')
    } else if (themeMode === 'light') {
      setThemeMode('dark')
    } else {
      setThemeMode('system')
    }
  }

  const theme = createTheme({
    palette: {
      mode,
      primary: {
        main: mode === 'dark' ? '#6366f1' : '#4f46e5',
      },
      secondary: {
        main: mode === 'dark' ? '#ec4899' : '#db2777',
      },
      background: mode === 'dark' ? {
        default: '#0f172a',
        paper: '#1e293b',
      } : {
        default: '#f8fafc',
        paper: '#ffffff',
      },
      text: mode === 'dark' ? {
        primary: '#f1f5f9',
        secondary: '#cbd5e1',
      } : {
        primary: '#0f172a',
        secondary: '#475569',
      },
    },
    typography: {
      fontFamily: '"Inter", "Roboto", "Helvetica", "Arial", sans-serif',
    },
    components: {
      MuiCssBaseline: {
        styleOverrides: {
          body: {
            scrollbarWidth: 'thin',
            '&::-webkit-scrollbar': {
              width: '8px',
              height: '8px',
            },
            '&::-webkit-scrollbar-thumb': {
              backgroundColor: mode === 'dark' ? '#475569' : '#cbd5e1',
              borderRadius: '4px',
            },
            '&::-webkit-scrollbar-track': {
              backgroundColor: mode === 'dark' ? '#1e293b' : '#f1f5f9',
            },
          },
        },
      },
      MuiPaper: {
        styleOverrides: {
          root: {
            backgroundImage: 'none',
            borderRadius: 12,
          },
        },
      },
      MuiButton: {
        styleOverrides: {
          root: {
            borderRadius: 8,
            textTransform: 'none',
            fontWeight: 500,
          },
          contained: {
            boxShadow: 'none',
            '&:hover': {
              boxShadow: mode === 'dark'
                ? '0 4px 12px rgba(99, 102, 241, 0.3)'
                : '0 4px 12px rgba(79, 70, 229, 0.2)',
            },
          },
        },
      },
      MuiTableRow: {
        styleOverrides: {
          root: {
            '&:hover': {
              backgroundColor: mode === 'dark' ? 'rgba(51, 65, 85, 0.8)' : 'rgba(241, 245, 249, 0.8)',
            },
          },
        },
      },
      MuiTableCell: {
        styleOverrides: {
          root: {
            borderBottom: mode === 'dark' ? '1px solid #334155' : '1px solid #e2e8f0',
          },
        },
      },
      MuiAppBar: {
        styleOverrides: {
          root: {
            boxShadow: mode === 'dark'
              ? '0 1px 3px 0 rgb(0 0 0 / 0.1)'
              : '0 1px 3px 0 rgb(0 0 0 / 0.05)',
          },
        },
      },
    },
  })

  return (
    <ThemeContext.Provider value={{ mode, themeMode, setThemeMode, toggleTheme }}>
      <ThemeProvider theme={theme}>
        {children}
      </ThemeProvider>
    </ThemeContext.Provider>
  )
}
