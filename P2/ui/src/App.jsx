import { Routes, Route, Navigate } from 'react-router-dom';
import { AuthProvider } from './context/AuthContext.jsx';
import { ToastProvider } from './context/ToastContext.jsx';
import { ProtectedRoute } from './components/ProtectedRoute.jsx';
import { Layout } from './components/Layout.jsx';
import { LoginPage } from './pages/LoginPage.jsx';
import { DashboardPage } from './pages/DashboardPage.jsx';
import { DomainDetailPage } from './pages/DomainDetailPage.jsx';
import { ZonalAuthPage } from './pages/ZonalAuthPage.jsx';
import { NotFoundPage } from './pages/NotFoundPage.jsx';

export default function App() {
  return (
    <AuthProvider>
      <ToastProvider>
        <Routes>
          <Route path="/" element={<Navigate to="/dashboard" replace />} />
          <Route path="/login" element={<LoginPage />} />
          <Route path="/auth/zonal" element={<ZonalAuthPage />} />

          <Route
            path="/dashboard"
            element={
              <ProtectedRoute>
                <Layout>
                  <DashboardPage />
                </Layout>
              </ProtectedRoute>
            }
          />
          <Route
            path="/domains/:id"
            element={
              <ProtectedRoute>
                <Layout>
                  <DomainDetailPage />
                </Layout>
              </ProtectedRoute>
            }
          />

          <Route path="*" element={<NotFoundPage />} />
        </Routes>
      </ToastProvider>
    </AuthProvider>
  );
}
