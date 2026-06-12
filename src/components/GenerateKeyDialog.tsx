import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  TextField,
  MenuItem,
  FormControl,
  InputLabel,
  Select,
  FormHelperText,
  Box,
} from '@mui/material'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

interface GenerateKeyDialogProps {
  open: boolean
  onClose: () => void
  onGenerate: (keyType: string, name: string, comment?: string, passphrase?: string, bits?: number) => Promise<void>
}

function GenerateKeyDialog({ open, onClose, onGenerate }: GenerateKeyDialogProps) {
  const [keyType, setKeyType] = useState('Ed25519')
  const [name, setName] = useState('')
  const [comment, setComment] = useState('')
  const [passphrase, setPassphrase] = useState('')
  const [bits, setBits] = useState('4096')
  const [loading, setLoading] = useState(false)
  const { t } = useTranslation()

  const handleSubmit = async () => {
    if (!name) return

    setLoading(true)
    try {
      await onGenerate(keyType, name, comment || undefined, passphrase || undefined, keyType === 'RSA' ? parseInt(bits) : undefined)
      // Parent component will close dialog via onClose in mutation's onSuccess
    } catch (error) {
      console.error('Failed to generate key:', error)
      setLoading(false)
    }
  }

  const handleClose = () => {
    setKeyType('Ed25519')
    setName('')
    setComment('')
    setPassphrase('')
    setBits('4096')
    onClose()
  }

  return (
    <Dialog open={open} onClose={handleClose} maxWidth="sm" fullWidth>
      <DialogTitle>{t('generateKeyDialog.title')}</DialogTitle>
      <DialogContent>
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2, mt: 1 }}>
          <FormControl fullWidth>
            <InputLabel>{t('generateKeyDialog.fieldKeyType')}</InputLabel>
            <Select
              value={keyType}
              label={t('generateKeyDialog.fieldKeyType')}
              onChange={(e) => setKeyType(e.target.value)}
            >
              <MenuItem value="Ed25519">{t('generateKeyDialog.keyTypeEd25519')}</MenuItem>
              <MenuItem value="RSA">{t('generateKeyDialog.keyTypeRSA')}</MenuItem>
              <MenuItem value="ECDSA">{t('generateKeyDialog.keyTypeECDSA')}</MenuItem>
              <MenuItem value="DSA">{t('generateKeyDialog.keyTypeDSA')}</MenuItem>
            </Select>
            <FormHelperText>
              {t('generateKeyDialog.keyTypeHelper')}
            </FormHelperText>
          </FormControl>

          {keyType === 'RSA' && (
            <FormControl fullWidth>
              <InputLabel>{t('generateKeyDialog.fieldBits')}</InputLabel>
              <Select
                value={bits}
                label={t('generateKeyDialog.fieldBits')}
                onChange={(e) => setBits(e.target.value)}
              >
                <MenuItem value="2048">2048</MenuItem>
                <MenuItem value="4096">4096</MenuItem>
              </Select>
              <FormHelperText>
                {t('generateKeyDialog.fieldBitsHelper')}
              </FormHelperText>
            </FormControl>
          )}

          <TextField
            fullWidth
            required
            label={t('generateKeyDialog.fieldName')}
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={t('generateKeyDialog.fieldNamePlaceholder')}
            helperText={t('generateKeyDialog.fieldNameHelper')}
          />

          <TextField
            fullWidth
            label={t('generateKeyDialog.fieldComment')}
            value={comment}
            onChange={(e) => setComment(e.target.value)}
            placeholder={t('generateKeyDialog.fieldCommentPlaceholder')}
            helperText={t('generateKeyDialog.fieldCommentHelper')}
          />

          <TextField
            fullWidth
            type="password"
            label={t('generateKeyDialog.fieldPassphrase')}
            value={passphrase}
            onChange={(e) => setPassphrase(e.target.value)}
            helperText={t('generateKeyDialog.fieldPassphraseHelper')}
          />
        </Box>
      </DialogContent>
      <DialogActions>
        <Button onClick={handleClose} disabled={loading}>
          {t('generateKeyDialog.cancel')}
        </Button>
        <Button
          onClick={handleSubmit}
          variant="contained"
          disabled={!name || loading}
        >
          {loading ? t('generateKeyDialog.generating') : t('generateKeyDialog.generate')}
        </Button>
      </DialogActions>
    </Dialog>
  )
}

export default GenerateKeyDialog
