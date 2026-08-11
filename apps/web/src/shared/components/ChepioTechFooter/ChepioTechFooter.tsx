function ChepioTechFooter() {
  return (
    <div className="bg-black/30 border-t border-white/5">
      <div className="max-w-7xl mx-auto px-6 lg:px-8 py-3 flex justify-end">
        <a
          href="https://chepio.tech"
          target="_blank"
          rel="noopener noreferrer"
          className="opacity-25 hover:opacity-100 transition-all duration-300"
          aria-label="Developed by Chepio"
        >
          <img
            src="/images/chepio-tech/logo_designed.svg"
            alt="chepio.tech"
            className="h-7 w-auto brightness-0 invert hover:brightness-100 hover:invert-0 transition-all duration-300"
          />
        </a>
      </div>
    </div>
  )
}

export default ChepioTechFooter
