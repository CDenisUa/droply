// Core
import { NavLink } from 'react-router-dom'

const linkClasses = ({ isActive }: { isActive: boolean }) =>
  `text-sm transition-colors ${isActive ? 'text-white' : 'text-white/50 hover:text-white/80'}`

function AppNav() {
  return (
    <nav className="flex items-center gap-6 border-b border-white/5 px-6 py-4">
      <span className="text-sm font-semibold tracking-tight text-white">Droply</span>
      <NavLink to="/" end className={linkClasses}>
        Home
      </NavLink>
      <NavLink to="/downloads" className={linkClasses}>
        Downloads
      </NavLink>
    </nav>
  )
}

export default AppNav
