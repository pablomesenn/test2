import { Link } from 'react-router-dom';
import { Button } from '../components/ui.jsx';
import { ArrowLeft } from 'lucide-react';

export function NotFoundPage() {
  return (
    <div className="min-h-screen flex items-center justify-center p-6">
      <div className="text-center max-w-md space-y-4 animate-fadeIn">
        <p className="mono text-xs tracking-[0.2em] uppercase text-ink-500">404 · No encontrado</p>
        <h1 className="heading-serif italic text-6xl text-ink-100">Aquí no hay nada.</h1>
        <p className="text-sm text-ink-400">
          La ruta que pediste no existe, o el recurso fue movido.
        </p>
        <Link to="/dashboard">
          <Button variant="secondary">
            <ArrowLeft className="w-4 h-4" />
            Volver al inicio
          </Button>
        </Link>
      </div>
    </div>
  );
}
