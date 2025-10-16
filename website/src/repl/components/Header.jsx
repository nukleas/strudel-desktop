import PlayCircleIcon from '@heroicons/react/20/solid/PlayCircleIcon';
import StopCircleIcon from '@heroicons/react/20/solid/StopCircleIcon';
import cx from '@src/cx.mjs';
import { useSettings, setIsZen } from '../../settings.mjs';
import '../Repl.css';

const { BASE_URL } = import.meta.env;
const baseNoTrailing = BASE_URL.endsWith('/') ? BASE_URL.slice(0, -1) : BASE_URL;

export function Header({ context, embedded = false }) {
  const { started, pending, isDirty, activeCode, handleTogglePlay, handleEvaluate, handleShuffle, handleShare } =
    context;
  const isEmbedded = typeof window !== 'undefined' && (embedded || window.location !== window.parent.location);
  const { isZen, isButtonRowHidden, isCSSAnimationDisabled, fontFamily } = useSettings();

  return (
    <>
    <header
      id="header"
      className={cx(
        'flex-none z-[100] text-lg select-none h-20 md:h-14',
        !isZen && !isEmbedded && 'bg-gradient-to-r from-[#0f172a] via-[#1e293b] to-[#0f172a] border-b border-[var(--border-cyan)] backdrop-blur-xl',
        isZen ? 'h-12 w-8 fixed top-0 left-0' : 'sticky top-0 w-full py-1 justify-between',
        isEmbedded ? 'flex' : 'md:flex',
      )}
      style={{ fontFamily }}
    >
      <div className="px-4 flex space-x-2 md:pt-0 select-none">
        <h1
          onClick={() => {
            if (isEmbedded) window.open(window.location.href.replace('embed', ''));
          }}
          className={cx(
            isEmbedded ? 'text-l cursor-pointer' : 'text-xl',
            'text-foreground font-bold flex space-x-2 items-center',
          )}
        >
          <div
            className={cx(
              'mt-[1px] relative',
              started && !isCSSAnimationDisabled && 'animate-spin',
              'cursor-pointer',
              isZen && 'fixed top-2 right-4',
            )}
            onClick={() => {
              if (!isEmbedded) {
                setIsZen(!isZen);
              }
            }}
          >
            <span className="block text-foreground rotate-90 icon-glow">꩜</span>
            {started && !isCSSAnimationDisabled && (
              <span className="absolute inset-0 bg-[var(--cyan-400)] opacity-20 blur-xl rounded-full"></span>
            )}
          </div>
          {!isZen && (
            <div className="space-x-2 flex items-baseline">
              <span className="gradient-text-cyan font-bold tracking-wide">strudel</span>
              <span className="text-sm font-medium text-[var(--foreground)] opacity-80">REPL</span>
              {!isEmbedded && isButtonRowHidden && (
                <a href={`${baseNoTrailing}/learn`} className="text-sm opacity-25 font-medium text-[var(--cyan-400)]">
                  DOCS
                </a>
              )}
              {started && (
                <span className="status-badge status-badge-live">
                  <span className="status-dot status-dot-live"></span>
                  LIVE
                </span>
              )}
              {pending && (
                <span className="status-badge status-badge-init">
                  <span className="status-dot status-dot-loading"></span>
                  LOADING
                </span>
              )}
            </div>
          )}
        </h1>
      </div>
      {!isZen && !isButtonRowHidden && (
        <div className="flex max-w-full overflow-auto text-foreground px-1 md:px-2">
          <button
            onClick={handleTogglePlay}
            title={started ? 'stop' : 'play'}
            className={cx(
              !isEmbedded ? 'px-4 py-2' : 'px-3 py-1',
              'rounded-md font-medium transition-all duration-200',
              started ? 'button-live' : 'button-cyberpunk',
              !started && !isCSSAnimationDisabled && 'animate-pulse',
            )}
          >
            {!pending ? (
              <span className={cx('flex items-center space-x-2')}>
                {started ? <StopCircleIcon className="w-5 h-5" /> : <PlayCircleIcon className="w-5 h-5" />}
                {!isEmbedded && <span className="uppercase tracking-wider text-sm font-mono">{started ? 'Stop' : 'Execute'}</span>}
              </span>
            ) : (
              <span className="uppercase tracking-wider text-sm font-mono">Loading...</span>
            )}
          </button>
          <button
            onClick={handleEvaluate}
            title="update"
            className={cx(
              'flex items-center space-x-1 rounded-md font-medium transition-all duration-200',
              !isEmbedded ? 'px-4 py-2' : 'px-3 py-1',
              'button-cyberpunk',
              !isDirty || !activeCode ? 'opacity-50 cursor-not-allowed' : '',
            )}
          >
            {!isEmbedded && <span className="uppercase tracking-wider text-sm font-mono">Update</span>}
          </button>
          {/* !isEmbedded && (
            <button
              title="shuffle"
              className="hover:opacity-50 p-2 flex items-center space-x-1"
              onClick={handleShuffle}
            >
              <span> shuffle</span>
            </button>
          ) */}
          {!isEmbedded && (
            <button
              title="share"
              className={cx(
                'cursor-pointer flex items-center space-x-1 rounded-md font-medium transition-all duration-200',
                !isEmbedded ? 'px-4 py-2' : 'px-3 py-1',
                'button-cyberpunk',
              )}
              onClick={handleShare}
            >
              <span className="uppercase tracking-wider text-sm font-mono">Share</span>
            </button>
          )}
          {!isEmbedded && (
            <a
              title="learn"
              href={`${baseNoTrailing}/workshop/getting-started/`}
              className={cx(
                'flex items-center space-x-1 rounded-md font-medium transition-all duration-200',
                !isEmbedded ? 'px-4 py-2' : 'px-3 py-1',
                'button-cyberpunk',
              )}
            >
              <span className="uppercase tracking-wider text-sm font-mono">Learn</span>
            </a>
          )}
          {/* {isEmbedded && (
            <button className={cx('hover:opacity-50 px-2')}>
              <a href={window.location.href} target="_blank" rel="noopener noreferrer" title="Open in REPL">
                🚀
              </a>
            </button>
          )}
          {isEmbedded && (
            <button className={cx('hover:opacity-50 px-2')}>
              <a
                onClick={() => {
                  window.location.href = initialUrl;
                  window.location.reload();
                }}
                title="Reset"
              >
                💔
              </a>
            </button>
          )} */}
        </div>
      )}
    </header>
    {!isZen && !isEmbedded && (
      <div className="header-gradient-border"></div>
    )}
    </>
  );
}
