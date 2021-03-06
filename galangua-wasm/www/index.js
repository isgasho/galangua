import {WasmAppFramework, WasmRenderer} from 'galangua-wasm'
import {audioManager} from './audio_manager'

const CHANNEL_COUNT = 3

const AUDIO_ASSETS = [
  'assets/audio/se_get_1',
  'assets/audio/se_pyuun',
  'assets/audio/se_zugyan',
  'assets/audio/jingle_1up',
]
const ENALBE_AUDIO = 'assets/audio/se_get_1'

const ICON_SOUND_ON = 'assets/imgs/sound_on.svg'
const ICON_SOUND_OFF = 'assets/imgs/sound_off.svg'

const CANVAS_ID = 'mycanvas'

window.play_se = function play_se(channel, filename) {
  audioManager.playSe(channel, filename)
}

function isTouchDevice() {
  try {
    document.createEvent("TouchEvent")
    return true
  } catch (e) {
    return false
  }
}

function fitCanvas() {
  const canvas = document.getElementById(CANVAS_ID)
  if (canvas.width / canvas.height >= window.innerWidth / window.innerHeight) {
    canvas.style.width = `100%`
    canvas.style.height = `${canvas.height * window.innerWidth / canvas.width}px`
  } else {
    canvas.style.height = `100%`
    canvas.style.width = `${canvas.width * window.innerHeight / canvas.height}px`
  }
}

function disableBounce() {
  document.addEventListener('touchmove', (event) => event.preventDefault(), {passive: false})
}

function setTouchHandler(id, callback) {
  const elem = document.getElementById(id)
  elem.addEventListener('touchstart', (e) => { /*e.preventDefault();*/ e.stopPropagation(); callback(true); return false; }, {passive: true})
  elem.addEventListener('touchend', (_) => callback(false))
  elem.addEventListener('touchleave', (_) => callback(false))
}

function setupTouchButtons() {
  const holder = document.getElementById('touch-btn-holder')

  if (isTouchDevice()) {
    // Touch enable
    setTouchHandler('shot-btn', (down) => framework.on_touch(100, down))
    holder.style.display = ''

    const stickGrip = document.getElementById('stick-grip')
    const stickArea = stickGrip.parentNode

    stickArea.addEventListener('touchstart', (event) => {
      // e.preventDefault()
      event.stopPropagation()

      if (event.changedTouches == null || event.changedTouches.length <= 0)
        return
      const id = event.changedTouches[0].identifier

      stickGrip.style.visibility = 'visible'
      const areaRect = stickArea.getBoundingClientRect()
      const gripRect = stickGrip.getBoundingClientRect()
      const w = gripRect.width
      const h = gripRect.height
      const D = areaRect.width / 6
      const l = areaRect.width / 2 - D
      const r = areaRect.width / 2 + D
      let lr = 0

      const findTargetTouch = (e) => {
        if (e.changedTouches != null) {
          for (let i = 0; i < e.changedTouches.length; ++i) {
            const touch = e.changedTouches[i]
            if (touch.identifier === id)
              return touch
          }
        }
        return null
      }

      const updatePosition = (e) => {
        const touch = findTargetTouch(e)
        if (touch != null) {
          const x = Math.min(Math.max(touch.clientX - areaRect.left, 0), areaRect.width)
          const y = Math.min(Math.max(touch.clientY - areaRect.top, 0), areaRect.height)
          stickGrip.style.left = `${Math.min(Math.max(x - w / 2, 0), areaRect.width - w)}px`
          stickGrip.style.top = `${Math.min(Math.max(y - h / 2, 0), areaRect.height - h)}px`

          lr = x <= l ? -1 : x >= r ? 1 : 0
          framework.on_touch(lr, true)
        }
      }

      const move = (e) => {
        updatePosition(e)
      }
      const end = (e) => {
        const touch = findTargetTouch(e)
        if (touch == null)
          return

        document.removeEventListener('touchmove', move)
        document.removeEventListener('touchend', end)
        document.removeEventListener('touchcancel', end)
        stickGrip.style.visibility = 'hidden'
        framework.on_touch(0, false)
      }
      document.addEventListener('touchmove', move)
      document.addEventListener('touchend', end)
      document.addEventListener('touchcancel', end)

      updatePosition(event)

      return false
    }, {passive: true})
  } else {
    // Touch disable
    holder.style.display = 'none'
  }

  const toggleSound = () => {
    audioManager.toggleEnabled()
    if (audioManager.enabled)
      audioManager.playSe(0, ENALBE_AUDIO)
    document.getElementById('sound-icon').src = audioManager.enabled ? ICON_SOUND_ON : ICON_SOUND_OFF
  }
  const soundIconHolder = document.getElementById('sound-icon-holder')
  soundIconHolder.addEventListener('click', toggleSound)
  soundIconHolder.addEventListener('touchstart', toggleSound)
}

function setupResizeListener() {
  window.addEventListener('resize', (_) => {
    fitCanvas()
  })
}

function createCoverScreen(title) {
  const cover = document.createElement('div')
  cover.className = 'centering'
  cover.style.position = 'absolute'
  cover.style.left = cover.style.top = cover.style.right = cover.style.bottom = '0'
  cover.style.backgroundColor = 'rgba(0,0,0,0.5)'
  cover.style.color = 'white'
  cover.style.textAlign = 'center'
  cover.innerText = title

  document.body.appendChild(cover)
  return cover
}

fitCanvas()
disableBounce()
setupResizeListener()

const renderer = WasmRenderer.new(CANVAS_ID)
const framework = WasmAppFramework.new(
  renderer, isTouchDevice(),
  function get_now() {
    return performance.now()
  },
  function get_item(key) {
    return localStorage.getItem(key)
  },
  function set_item(key, value) {
    localStorage.setItem(key, value)
  })

document.addEventListener('keydown', (event) => {
  framework.on_key(event.code, true)
})
document.addEventListener('keyup', (event) => {
  framework.on_key(event.code, false)
})

const loop = (function() {
  const target_fps = 60
  const ticks = 1000 / target_fps
  const max_skip = 5
  const margin = ticks / 8

  let prev = performance.now()
  return function loop() {
    const now = performance.now()
    let n = Math.floor((now - prev + margin) / ticks)
    if (n > 0) {
      if (n <= max_skip) {
        prev += n * ticks
      } else {
        n = max_skip
        prev = now
      }
      for (let i = 0; i < n; ++i)
        framework.update()
      framework.draw()
    }
    requestAnimationFrame(loop)
  }
})()

const cover = createCoverScreen('Loading...')
audioManager.createContext(CHANNEL_COUNT)
audioManager.loadAllAudios(AUDIO_ASSETS)
  .then(() => {
    document.body.removeChild(cover)
    setupTouchButtons()
    requestAnimationFrame(loop)
  })

document.documentElement.addEventListener('touchend', (event) => {
  event.preventDefault()
}, false)
