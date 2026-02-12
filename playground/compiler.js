// HRML Playground — WASM compiler bridge
let hrmlWasm = null;
let debounceTimer = null;

export async function initCompiler() {
  const mod = await import('./pkg/hrml_wasm.js');
  await mod.default();
  hrmlWasm = mod;
  return hrmlWasm;
}

export function getVersion() {
  return hrmlWasm ? hrmlWasm.version() : '';
}

export function compile(source) {
  if (!hrmlWasm) return { error: 'Compiler not loaded yet' };
  try {
    const result = hrmlWasm.compile(source);
    return { html: result.html, css: result.css, js: result.js };
  } catch (e) {
    return { error: e.message || String(e) };
  }
}

export function compileAndRender(source, previewFrame, outputPanel) {
  const result = compile(source);

  if (result.error) {
    outputPanel.textContent = result.error;
    outputPanel.classList.add('error');
    return;
  }

  outputPanel.classList.remove('error');

  // Build standalone HTML for the iframe (compact — no extra indentation)
  const iframeDoc = '<!DOCTYPE html>\n<html>\n<head>\n<meta charset="UTF-8">\n<style>\n'
    + '  body { font-family: system-ui, sans-serif; padding: 16px; margin: 0; }\n'
    + '  button { padding: 6px 12px; cursor: pointer; }\n'
    + '  input { padding: 6px; }\n'
    + '  .counter { display: flex; align-items: center; gap: 8px; }\n'
    + (result.css ? result.css + '\n' : '')
    + '</style>\n</head>\n<body>\n'
    + result.html
    + (result.js ? '<script>\n' + result.js + '</script>\n' : '')
    + '</body>\n</html>';

  previewFrame.srcdoc = iframeDoc;

  // Build well-indented output for display
  const output = buildFormattedOutput(result);
  outputPanel.innerHTML = output;
}

function buildFormattedOutput(result) {
  let lines = [];

  lines.push('<!DOCTYPE html>');
  lines.push('<html>');
  lines.push('<head>');
  lines.push('  <meta charset="UTF-8">');

  if (result.css) {
    lines.push('  <style>');
    result.css.split('\n').forEach(l => lines.push('    ' + l));
    lines.push('  </style>');
  }

  lines.push('</head>');
  lines.push('<body>');

  // Indent HTML body content by 2 spaces
  result.html.split('\n').forEach(l => {
    if (l.trim()) lines.push('  ' + l);
  });

  if (result.js) {
    lines.push('  <script>');
    result.js.split('\n').forEach(l => {
      if (l.trim()) lines.push('    ' + l);
    });
    lines.push('  </script>');
  }

  lines.push('</body>');
  lines.push('</html>');

  return highlight(lines.join('\n'));
}

// =========================================================================
// Syntax highlighting
// =========================================================================

function esc(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function highlight(code) {
  const escaped = esc(code);

  return escaped
    // HTML comments
    .replace(/(&lt;!--.*?--&gt;)/g, '<span class="hl-comment">$1</span>')
    // DOCTYPE
    .replace(/(&lt;!DOCTYPE\s+\w+&gt;)/gi, '<span class="hl-doctype">$1</span>')
    // HTML closing tags
    .replace(/(&lt;\/\w+&gt;)/g, '<span class="hl-tag">$1</span>')
    // HTML opening tags (with attributes)
    .replace(/(&lt;\w+)((\s+[^&]*?)*?)(&gt;)/g, (_, open, attrs, _2, close) => {
      const coloredAttrs = attrs
        // id="..." class="..."
        .replace(/(\w[\w-]*)(\s*=\s*)(&quot;[^&]*?&quot;|&#39;[^&]*?&#39;|\S+)/g,
          '<span class="hl-attr-name">$1</span>$2<span class="hl-attr-value">$3</span>')
        // charset, etc (no value)
        .replace(/(\s)(\w[\w-]*)(?=\s|$)/g, '$1<span class="hl-attr-name">$2</span>');
      return '<span class="hl-tag">' + open + '</span>' + coloredAttrs + '<span class="hl-tag">' + close + '</span>';
    })
    // JS strings in script blocks
    .replace(/(&#39;[^&#]*?&#39;)/g, '<span class="hl-string">$1</span>')
    .replace(/(`[^`]*?`)/g, '<span class="hl-string">$1</span>')
    // JS keywords
    .replace(/\b(const|let|var|function|return|if|else|new|true|false|null)\b/g,
      '<span class="hl-keyword">$1</span>')
    // JS comments
    .replace(/(\/\/.*?)$/gm, '<span class="hl-comment">$1</span>')
    // Numbers
    .replace(/\b(\d+\.?\d*)\b/g, '<span class="hl-number">$1</span>');
}

export function scheduleCompile(source, previewFrame, outputPanel, delay) {
  clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    compileAndRender(source, previewFrame, outputPanel);
  }, delay);
}
