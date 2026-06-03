import { createContext, useContext, useEffect, useState } from 'react';
import { watchAuth, signOut as fbSignOut } from '../lib/firebase.js';
import { isFirebaseConfigured } from '../config.js';

const AuthContext = createContext({
  user: null,
  ready: false,
  signOut: async () => {},
  firebaseConfigured: false,
});

export function AuthProvider({ children }) {
  const [user, setUser] = useState(null);
  const [ready, setReady] = useState(false);
  const firebaseConfigured = isFirebaseConfigured();

  useEffect(() => {
    if (!firebaseConfigured) {
      setReady(true);
      return undefined;
    }
    const unsub = watchAuth((u) => {
      setUser(u);
      setReady(true);
    });
    return unsub;
  }, [firebaseConfigured]);

  const value = {
    user,
    ready,
    firebaseConfigured,
    signOut: async () => {
      await fbSignOut();
      setUser(null);
    },
  };

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  return useContext(AuthContext);
}
