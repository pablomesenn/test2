import { Navigate, useLocation } from 'react-router-dom';
import { useAuth } from '../context/AuthContext.jsx';
import { Spinner } from './ui.jsx';

export function ProtectedRoute({ children }) {
  const { user, ready, firebaseConfigured } = useAuth();
  const location = useLocation();

  if (!firebaseConfigured) {
    return <Navigate to="/login" replace state={{ from: location }} />;
  }

  if (!ready) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <Spinner label="Cargando sesión…" />
      </div>
    );
  }

  if (!user) {
    return <Navigate to="/login" replace state={{ from: location }} />;
  }

  return children;
}
