import { useMemo, useCallback } from 'react'
import Particles, { ParticlesProvider } from '@tsparticles/react'
import { loadSlim } from '@tsparticles/slim'
import { useTheme } from '../ThemeContext.jsx'

export default function Web3Background() {
  const { theme } = useTheme()
  const isDark = theme === 'dark'

  const init = useCallback(async (engine) => {
    await loadSlim(engine)
  }, [])

  const options = useMemo(
    () => ({
      fpsLimit: 60,
      detectRetina: true,
      background: { color: { value: 'transparent' } },
      interactivity: {
        events: {
          onHover: { enable: true, mode: 'grab' },
        },
        modes: {
          grab: { distance: 150, links: { opacity: 0.45 } },
        },
      },
      particles: {
        number: { value: 42, density: { enable: true, area: 1000 } },
        color: { value: isDark ? ['#38bdf8', '#818cf8', '#34d399'] : ['#2563eb', '#0891b2', '#6366f1'] },
        links: {
          enable: true,
          distance: 155,
          color: isDark ? '#38bdf8' : '#3b82f6',
          opacity: isDark ? 0.16 : 0.12,
          width: 1,
        },
        move: {
          enable: true,
          speed: 0.55,
          direction: 'none',
          outModes: { default: 'bounce' },
        },
        opacity: { value: isDark ? 0.45 : 0.3 },
        size: { value: { min: 1.2, max: 2.6 } },
      },
    }),
    [isDark],
  )

  return (
    <ParticlesProvider init={init}>
      <Particles
        id="nebula-web3-mesh"
        options={options}
        className="pointer-events-none fixed inset-0 z-0"
      />
    </ParticlesProvider>
  )
}
