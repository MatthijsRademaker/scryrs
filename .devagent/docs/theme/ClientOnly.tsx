import { useEffect, useState, type ReactNode } from 'react';

export default function ClientOnly({ children }: { children: ReactNode }) {
  const [mounted, setMounted] = useState(false);
  const [scriptLoaded, setScriptLoaded] = useState(false);
  const [scriptError, setScriptError] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  useEffect(() => {
    if (!mounted) return;
    if (document.querySelector('script[data-likec4]')) {
      setScriptLoaded(true);
      return;
    }
    const script = document.createElement('script');
    script.type = 'module';
    script.src = '/project-docs/likec4-webcomponent.js';
    script.setAttribute('data-likec4', '');
    script.onload = () => setScriptLoaded(true);
    script.onerror = () => setScriptError(true);
    document.head.appendChild(script);
  }, [mounted]);

  if (scriptError) {
    return <div style={{ padding: '2rem', color: '#e06060', fontSize: '14px' }}>Failed to load architecture diagrams.</div>;
  }

  if (!mounted || !scriptLoaded) {
    return <div style={{ padding: '2rem', color: '#7888a0', fontSize: '14px' }}>Loading architecture diagrams...</div>;
  }

  return <>{children}</>;
}
