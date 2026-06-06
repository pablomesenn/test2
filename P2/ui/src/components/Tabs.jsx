export function Tabs({ tabs, current, onChange }) {
  return (
    <div className="border-b border-ink-800">
      <nav className="flex gap-1 -mb-px" role="tablist">
        {tabs.map((tab) => {
          const active = tab.value === current;
          const Icon = tab.icon;
          return (
            <button
              key={tab.value}
              role="tab"
              aria-selected={active}
              onClick={() => onChange(tab.value)}
              className={[
                'inline-flex items-center gap-2 px-4 py-2.5 text-sm border-b-2 transition-colors',
                active
                  ? 'border-accent-500 text-ink-100'
                  : 'border-transparent text-ink-400 hover:text-ink-200 hover:border-ink-700',
              ].join(' ')}
            >
              {Icon && <Icon className="w-4 h-4" />}
              <span>{tab.label}</span>
              {tab.count !== undefined && (
                <span className="text-[10px] mono px-1.5 py-0.5 rounded bg-ink-800 text-ink-400">
                  {tab.count}
                </span>
              )}
            </button>
          );
        })}
      </nav>
    </div>
  );
}
