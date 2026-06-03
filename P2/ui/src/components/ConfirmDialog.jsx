import { Modal } from './Modal.jsx';
import { Button } from './ui.jsx';
import { AlertTriangle } from 'lucide-react';

export function ConfirmDialog({
  open,
  onClose,
  onConfirm,
  title = '¿Estás seguro?',
  description,
  confirmLabel = 'Eliminar',
  confirmVariant = 'danger',
  loading = false,
}) {
  return (
    <Modal
      open={open}
      onClose={onClose}
      title={title}
      size="sm"
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={loading}>
            Cancelar
          </Button>
          <Button variant={confirmVariant} onClick={onConfirm} loading={loading}>
            {confirmLabel}
          </Button>
        </>
      }
    >
      <div className="flex items-start gap-3">
        <div className="p-2 rounded-full bg-danger-500/10 text-danger-500 border border-danger-500/30 shrink-0">
          <AlertTriangle className="w-4 h-4" />
        </div>
        <p className="text-sm text-ink-300">{description}</p>
      </div>
    </Modal>
  );
}
