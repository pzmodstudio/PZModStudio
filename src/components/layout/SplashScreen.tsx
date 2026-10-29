import React, { useEffect, useState, useCallback } from 'react';
import { Terminal, Cpu, CheckCircle2, ShieldAlert, MousePointerClick } from 'lucide-react';
import i18next from 'i18next';

interface SplashScreenProps {
  isLoading: boolean;
  statusMessage: string;
  progress: number;
}

const CINEMATIC_PHRASES = [
  { text: 'line1', color: 'text-slate-100', glow: 'drop-shadow-[0_0_20px_rgba(255,255,255,0.45)]' },
  { text: 'line2', color: 'text-slate-200', glow: 'drop-shadow-[0_0_20px_rgba(226,232,240,0.35)]' },
  { text: 'line3', color: 'text-red-500 font-bold', glow: 'drop-shadow-[0_0_35px_rgba(239,68,68,0.95)]' },
];

export const SplashScreen: React.FC<SplashScreenProps> = ({
  isLoading,
  statusMessage,
  progress,
}) => {
  const [currentPhraseIndex, setCurrentPhraseIndex] = useState(0);
  const [phraseVisible, setPhraseVisible] = useState(true);
  /** The three phrases have played: the last one stays, with "press any key" under it. */
  const [introDone, setIntroDone] = useState(false);
  /** A key or click came while the app was still loading: leave as soon as it is ready. */
  const [wantsOut, setWantsOut] = useState(false);
  const [shouldRender, setShouldRender] = useState(true);
  const [isFadingOut, setIsFadingOut] = useState(false);

  // Dismiss logic
  const handleDismiss = useCallback(() => {
    if (!isFadingOut) {
      setIsFadingOut(true);
      setTimeout(() => {
        setShouldRender(false);
      }, 700);
    }
  }, [isFadingOut]);

  /** Like the game's intro: nothing leaves on its own; a key or a click does, at any moment. */
  const requestDismiss = useCallback(() => {
    if (isLoading) setWantsOut(true);
    else handleDismiss();
  }, [isLoading, handleDismiss]);

  // Cinematic phrase sequence controller (Authentic, suspenseful PZ timing)
  useEffect(() => {
    // Phrase 1 (0s -> 3.2s) -> Phrase 2
    const t1 = setTimeout(() => {
      setPhraseVisible(false);
      setTimeout(() => {
        setCurrentPhraseIndex(1);
        setPhraseVisible(true);
      }, 600);
    }, 3200);

    // Phrase 2 (3.8s -> 7.0s) -> Phrase 3
    const t2 = setTimeout(() => {
      setPhraseVisible(false);
      setTimeout(() => {
        setCurrentPhraseIndex(2);
        setPhraseVisible(true);
      }, 600);
    }, 7000);

    // Phrase 3 fades in at 7.6s and is fully on screen a second later: it stays there.
    const t3 = setTimeout(() => {
      setIntroDone(true);
    }, 8800);

    return () => {
      clearTimeout(t1);
      clearTimeout(t2);
      clearTimeout(t3);
    };
  }, []);

  // Asked to leave while loading: go once the app is ready.
  useEffect(() => {
    if (wantsOut && !isLoading) handleDismiss();
  }, [wantsOut, isLoading, handleDismiss]);

  // Any key leaves.
  useEffect(() => {
    const handleKeyDown = () => requestDismiss();
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [requestDismiss]);

  if (!shouldRender) return null;

  const currentPhrase = CINEMATIC_PHRASES[currentPhraseIndex];

  return (
    <div
      onClick={requestDismiss}
      className={`fixed inset-0 z-[9999] flex flex-col justify-between items-center bg-black text-slate-100 select-none transition-all duration-700 ease-in-out cursor-pointer ${
        isFadingOut ? 'opacity-0 scale-105 pointer-events-none' : 'opacity-100 scale-100'
      }`}
      style={{
        backgroundImage: 'radial-gradient(ellipse 75% 55% at 50% 45%, rgba(185, 28, 28, 0.1) 0%, rgba(0, 0, 0, 1) 100%)',
      }}
    >
      {/* Top Subtle Ambient Header */}
      <div className="w-full pt-8 px-8 flex justify-between items-center text-[11px] font-mono text-neutral-600 tracking-wider uppercase">
        <div className="flex items-center gap-2">
          <span className="w-1.5 h-1.5 rounded-full bg-red-600 animate-ping" />
          <span>Project Zomboid Mod Studio</span>
        </div>
        <div>v{__APP_VERSION__} • Standalone</div>
      </div>

      {/* Main Center Area: Iconic Project Zomboid Intro Typography */}
      <div className="flex flex-col items-center justify-center px-8 max-w-3xl text-center my-auto">
        <div
          className={`transition-all duration-1000 ease-out transform ${
            phraseVisible ? 'opacity-100 translate-y-0 scale-100' : 'opacity-0 translate-y-3 scale-95'
          }`}
        >
          <h2
            className={`text-3xl sm:text-5xl md:text-6xl font-serif tracking-widest leading-relaxed select-none ${currentPhrase.color} ${currentPhrase.glow}`}
            style={{
              fontFamily: 'Georgia, Cambria, "Times New Roman", Times, serif',
              letterSpacing: '0.09em',
            }}
          >
            "{i18next.t(`splash.${currentPhrase.text}`)}"
          </h2>
        </div>

        {/* "Press any key", once the phrases have played and the app is ready; it stays. */}
        <div
          className={`mt-10 transition-opacity duration-700 flex items-center justify-center gap-2 text-xs font-mono text-neutral-400 tracking-widest uppercase ${
            introDone && !isLoading ? 'opacity-100 animate-pulse' : 'opacity-0'
          }`}
        >
          <MousePointerClick className="w-4 h-4 text-red-500" />
          <span>{i18next.t('splash.continue')}</span>
        </div>
      </div>

      {/* Bottom Area: Real-Time Diagnostic & Subsystem Loading Panel */}
      <div className="w-full max-w-xl pb-10 px-6 space-y-3">
        <div className="bg-neutral-950/95 border border-neutral-800/90 rounded-xl p-4 shadow-2xl backdrop-blur-md space-y-2.5">
          {/* Header of real loading monitor */}
          <div className="flex items-center justify-between text-[11px] font-mono text-neutral-400 border-b border-neutral-900 pb-2">
            <div className="flex items-center gap-2 text-neutral-300">
              <Terminal className="w-3.5 h-3.5 text-red-500" />
              <span className="font-semibold tracking-wide text-neutral-300">REAL LOADING STATUS</span>
            </div>
            <span className="text-red-400 font-bold font-mono">{Math.round(progress)}%</span>
          </div>

          {/* Real-time status string showing actual activity */}
          <div className="flex items-center justify-between gap-3 text-xs font-mono">
            <div className="flex items-center gap-2.5 min-w-0">
              <span className="relative flex h-2 w-2 shrink-0">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-red-400 opacity-75"></span>
                <span className="relative inline-flex rounded-full h-2 w-2 bg-red-500"></span>
              </span>
              <span className="text-neutral-300 truncate text-[11px] sm:text-xs font-mono">
                {statusMessage}
              </span>
            </div>
          </div>

          {/* Visual Progress Bar */}
          <div className="w-full h-1.5 bg-neutral-900 border border-neutral-800 rounded-full overflow-hidden p-0">
            <div
              className="h-full bg-gradient-to-r from-neutral-700 via-red-600 to-red-500 rounded-full transition-all duration-300 ease-out shadow-[0_0_10px_rgba(239,68,68,0.6)]"
              style={{ width: `${Math.max(8, Math.min(100, progress))}%` }}
            />
          </div>

          {/* Subsystems Diagnostic Tickers */}
          <div className="flex items-center justify-between text-[10px] font-mono text-neutral-500 pt-0.5">
            <span className="flex items-center gap-1">
              <Cpu className="w-3 h-3 text-neutral-600" />
              <span>{i18next.t('splash.lua')}</span>
            </span>
            <span className="flex items-center gap-1">
              <ShieldAlert className="w-3 h-3 text-neutral-600" />
              <span>{i18next.t('splash.conflicts')}</span>
            </span>
            <span className="flex items-center gap-1 text-emerald-500/80">
              <CheckCircle2 className="w-3 h-3 text-emerald-500" />
              <span>{i18next.t('splash.ready')}</span>
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};
