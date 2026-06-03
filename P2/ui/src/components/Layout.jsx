import { NavLink, useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext.jsx';
import { Button, MonoTag } from './ui.jsx';
import { LogOut, Network, Home } from 'lucide-react';
import { config } from '../config.js';

export function Layout({ children }) {
  const { user, signOut } = useAuth();
  const navigate = useNavigate();

  const onSignOut = async () => {
    await signOut();
    navigate('/login', { replace: true });
  };

  return (
    <div className="min-h-screen flex flex-col">
      <header className="border-b border-ink-800 bg-ink-950/80 backdrop-blur sticky top-0 z-30">
        <div className="max-w-7xl mx-auto px-6 py-3 flex items-center gap-4">
          <NavLink to="/dashboard" className="flex items-center gap-2.5 group">
            <div className="relative">
              <div className="w-7 h-7 rounded-md border border-accent-500/40 bg-accent-500/10 flex items-center justify-center">
                <Network className="w-3.5 h-3.5 text-accent-500" />
              </div>
              <div className="absolute inset-0 rounded-md bg-accent-500/20 blur-md opacity-0 group-hover:opacity-100 transition-opacity" />
            </div>
            <div className="leading-none">
              <span className="heading-serif italic text-lg text-ink-100">Zonal Cache</span>
              <span className="block text-[10px] tracking-[0.18em] text-ink-500 uppercase mt-0.5">
                Consola · IC-7602
              </span>
            </div>
          </NavLink>

          <nav className="ml-6 hidden md:flex items-center gap-1">
            <NavItem to="/dashboard" icon={Home} label="Dominios" />
          </nav>

          <div className="ml-auto flex items-center gap-3">
            <div className="hidden sm:block text-right">
              <p className="text-xs text-ink-300 truncate max-w-[180px]">{user?.email}</p>
              <p className="text-[10px] text-ink-500 mono truncate max-w-[180px]">
                uid {user?.uid?.slice(0, 8)}…
              </p>
            </div>
            <Button variant="ghost" size="sm" onClick={onSignOut}>
              <LogOut className="w-3.5 h-3.5" />
              Cerrar sesión
            </Button>
          </div>
        </div>
      </header>

      <main className="flex-1">
        <div className="max-w-7xl mx-auto px-6 py-8 animate-fadeIn">{children}</div>
      </main>

      <footer className="border-t border-ink-800 mt-12">
        <div className="max-w-7xl mx-auto px-6 py-4 flex flex-wrap items-center justify-between gap-2 text-xs text-ink-500">
          <span>Tecnológico de Costa Rica · Redes (IC-7602) · Proyecto 2</span>
          <span className="flex items-center gap-2">
            <span>API</span>
            <MonoTag>{new URL(config.apiUrl).host}</MonoTag>
          </span>
        </div>
      </footer>
    </div>
  );
}

function NavItem({ to, icon: Icon, label }) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        [
          'inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm transition-colors',
          isActive ? 'bg-ink-800 text-ink-100' : 'text-ink-400 hover:text-ink-200 hover:bg-ink-900',
        ].join(' ')
      }
    >
      {Icon && <Icon className="w-3.5 h-3.5" />}
      {label}
    </NavLink>
  );
}
