(function() {var type_impls = {
"io_lifetimes":[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-OwnedFd\" class=\"impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#82\">source</a><a href=\"#impl-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.try_clone\" class=\"method\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.63.0\">1.63.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#86\">source</a></span><h4 class=\"code-header\">pub fn <a href=\"io_lifetimes/struct.OwnedFd.html#tymethod.try_clone\" class=\"fn\">try_clone</a>(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.77.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/io/error/struct.Error.html\" title=\"struct std::io::error::Error\">Error</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Creates a new <code>OwnedFd</code> instance that shares the same underlying file\ndescription as the existing <code>OwnedFd</code> instance.</p>\n</div></details></div></details>",0,"io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-AsFd-for-OwnedFd\" class=\"impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.63.0\">1.63.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#271\">source</a></span><a href=\"#impl-AsFd-for-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"io_lifetimes/trait.AsFd.html\" title=\"trait io_lifetimes::AsFd\">AsFd</a> for <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.as_fd\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#273\">source</a><a href=\"#method.as_fd\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"io_lifetimes/trait.AsFd.html#tymethod.as_fd\" class=\"fn\">as_fd</a>(&amp;self) -&gt; <a class=\"struct\" href=\"io_lifetimes/struct.BorrowedFd.html\" title=\"struct io_lifetimes::BorrowedFd\">BorrowedFd</a>&lt;'_&gt;</h4></section></summary><div class='docblock'>Borrows the file descriptor. <a href=\"io_lifetimes/trait.AsFd.html#tymethod.as_fd\">Read more</a></div></details></div></details>","AsFd","io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-FromRawFd-for-OwnedFd\" class=\"impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.63.0\">1.63.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#154\">source</a></span><a href=\"#impl-FromRawFd-for-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.77.0/std/os/fd/raw/trait.FromRawFd.html\" title=\"trait std::os::fd::raw::FromRawFd\">FromRawFd</a> for <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from_raw_fd\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#164\">source</a><a href=\"#method.from_raw_fd\" class=\"anchor\">§</a><h4 class=\"code-header\">unsafe fn <a href=\"https://doc.rust-lang.org/1.77.0/std/os/fd/raw/trait.FromRawFd.html#tymethod.from_raw_fd\" class=\"fn\">from_raw_fd</a>(fd: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.77.0/std/primitive.i32.html\">i32</a>) -&gt; <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h4></section></summary><div class=\"docblock\"><p>Constructs a new instance of <code>Self</code> from the given raw file descriptor.</p>\n<h5 id=\"safety\"><a class=\"doc-anchor\" href=\"#safety\">§</a>Safety</h5>\n<p>The resource pointed to by <code>fd</code> must be open and suitable for assuming\n<a href=\"https://doc.rust-lang.org/1.77.0/std/io/index.html#io-safety\" title=\"mod std::io\">ownership</a>. The resource must not require any cleanup other than <code>close</code>.</p>\n</div></details></div></details>","FromRawFd","io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Drop-for-OwnedFd\" class=\"impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.63.0\">1.63.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#172\">source</a></span><a href=\"#impl-Drop-for-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.77.0/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.drop\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#174\">source</a><a href=\"#method.drop\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.77.0/core/ops/drop/trait.Drop.html#tymethod.drop\" class=\"fn\">drop</a>(&amp;mut self)</h4></section></summary><div class='docblock'>Executes the destructor for this type. <a href=\"https://doc.rust-lang.org/1.77.0/core/ops/drop/trait.Drop.html#tymethod.drop\">Read more</a></div></details></div></details>","Drop","io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-OwnedFd\" class=\"impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.63.0\">1.63.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#197\">source</a></span><a href=\"#impl-Debug-for-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.77.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#198\">source</a><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.77.0/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.77.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.77.0/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/core/fmt/struct.Error.html\" title=\"struct core::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.77.0/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-From%3CChildStdout%3E-for-OwnedFd\" class=\"impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.63.0\">1.63.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/unix/process.rs.html#460\">source</a></span><a href=\"#impl-From%3CChildStdout%3E-for-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/process/struct.ChildStdout.html\" title=\"struct std::process::ChildStdout\">ChildStdout</a>&gt; for <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/unix/process.rs.html#462\">source</a><a href=\"#method.from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html#tymethod.from\" class=\"fn\">from</a>(child_stdout: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/process/struct.ChildStdout.html\" title=\"struct std::process::ChildStdout\">ChildStdout</a>) -&gt; <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h4></section></summary><div class='docblock'>Converts to this type from the input type.</div></details></div></details>","From<ChildStdout>","io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-From%3CChildStderr%3E-for-OwnedFd\" class=\"impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.63.0\">1.63.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/unix/process.rs.html#490\">source</a></span><a href=\"#impl-From%3CChildStderr%3E-for-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/process/struct.ChildStderr.html\" title=\"struct std::process::ChildStderr\">ChildStderr</a>&gt; for <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/unix/process.rs.html#492\">source</a><a href=\"#method.from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html#tymethod.from\" class=\"fn\">from</a>(child_stderr: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/process/struct.ChildStderr.html\" title=\"struct std::process::ChildStderr\">ChildStderr</a>) -&gt; <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h4></section></summary><div class='docblock'>Converts to this type from the input type.</div></details></div></details>","From<ChildStderr>","io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-From%3CFile%3E-for-OwnedFd\" class=\"impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.63.0\">1.63.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#290\">source</a></span><a href=\"#impl-From%3CFile%3E-for-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/fs/struct.File.html\" title=\"struct std::fs::File\">File</a>&gt; for <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#292\">source</a><a href=\"#method.from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html#tymethod.from\" class=\"fn\">from</a>(file: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/fs/struct.File.html\" title=\"struct std::fs::File\">File</a>) -&gt; <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h4></section></summary><div class='docblock'>Converts to this type from the input type.</div></details></div></details>","From<File>","io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-From%3CTcpStream%3E-for-OwnedFd\" class=\"impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.63.0\">1.63.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#314\">source</a></span><a href=\"#impl-From%3CTcpStream%3E-for-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/net/tcp/struct.TcpStream.html\" title=\"struct std::net::tcp::TcpStream\">TcpStream</a>&gt; for <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#316\">source</a><a href=\"#method.from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html#tymethod.from\" class=\"fn\">from</a>(tcp_stream: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/net/tcp/struct.TcpStream.html\" title=\"struct std::net::tcp::TcpStream\">TcpStream</a>) -&gt; <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h4></section></summary><div class='docblock'>Converts to this type from the input type.</div></details></div></details>","From<TcpStream>","io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-From%3CPidFd%3E-for-OwnedFd\" class=\"impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/linux/process.rs.html#104\">source</a><a href=\"#impl-From%3CPidFd%3E-for-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/os/linux/process/struct.PidFd.html\" title=\"struct std::os::linux::process::PidFd\">PidFd</a>&gt; for <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/linux/process.rs.html#105\">source</a><a href=\"#method.from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html#tymethod.from\" class=\"fn\">from</a>(pid_fd: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/os/linux/process/struct.PidFd.html\" title=\"struct std::os::linux::process::PidFd\">PidFd</a>) -&gt; <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h4></section></summary><div class='docblock'>Converts to this type from the input type.</div></details></div></details>","From<PidFd>","io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-From%3CUdpSocket%3E-for-OwnedFd\" class=\"impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.63.0\">1.63.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#366\">source</a></span><a href=\"#impl-From%3CUdpSocket%3E-for-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/net/udp/struct.UdpSocket.html\" title=\"struct std::net::udp::UdpSocket\">UdpSocket</a>&gt; for <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#368\">source</a><a href=\"#method.from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html#tymethod.from\" class=\"fn\">from</a>(udp_socket: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/net/udp/struct.UdpSocket.html\" title=\"struct std::net::udp::UdpSocket\">UdpSocket</a>) -&gt; <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h4></section></summary><div class='docblock'>Converts to this type from the input type.</div></details></div></details>","From<UdpSocket>","io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-From%3CUnixDatagram%3E-for-OwnedFd\" class=\"impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.63.0\">1.63.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/unix/net/datagram.rs.html#1026\">source</a></span><a href=\"#impl-From%3CUnixDatagram%3E-for-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/os/unix/net/datagram/struct.UnixDatagram.html\" title=\"struct std::os::unix::net::datagram::UnixDatagram\">UnixDatagram</a>&gt; for <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/unix/net/datagram.rs.html#1028\">source</a><a href=\"#method.from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html#tymethod.from\" class=\"fn\">from</a>(unix_datagram: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/os/unix/net/datagram/struct.UnixDatagram.html\" title=\"struct std::os::unix::net::datagram::UnixDatagram\">UnixDatagram</a>) -&gt; <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h4></section></summary><div class='docblock'>Converts to this type from the input type.</div></details></div></details>","From<UnixDatagram>","io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-From%3CUnixStream%3E-for-OwnedFd\" class=\"impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.63.0\">1.63.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/unix/net/stream.rs.html#754\">source</a></span><a href=\"#impl-From%3CUnixStream%3E-for-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/os/unix/net/stream/struct.UnixStream.html\" title=\"struct std::os::unix::net::stream::UnixStream\">UnixStream</a>&gt; for <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/unix/net/stream.rs.html#756\">source</a><a href=\"#method.from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html#tymethod.from\" class=\"fn\">from</a>(unix_stream: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/os/unix/net/stream/struct.UnixStream.html\" title=\"struct std::os::unix::net::stream::UnixStream\">UnixStream</a>) -&gt; <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h4></section></summary><div class='docblock'>Converts to this type from the input type.</div></details></div></details>","From<UnixStream>","io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-From%3CChildStdin%3E-for-OwnedFd\" class=\"impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.63.0\">1.63.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/unix/process.rs.html#430\">source</a></span><a href=\"#impl-From%3CChildStdin%3E-for-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/process/struct.ChildStdin.html\" title=\"struct std::process::ChildStdin\">ChildStdin</a>&gt; for <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/unix/process.rs.html#432\">source</a><a href=\"#method.from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html#tymethod.from\" class=\"fn\">from</a>(child_stdin: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/process/struct.ChildStdin.html\" title=\"struct std::process::ChildStdin\">ChildStdin</a>) -&gt; <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h4></section></summary><div class='docblock'>Converts to this type from the input type.</div></details></div></details>","From<ChildStdin>","io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-From%3CUnixListener%3E-for-OwnedFd\" class=\"impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.63.0\">1.63.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/unix/net/listener.rs.html#348\">source</a></span><a href=\"#impl-From%3CUnixListener%3E-for-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/os/unix/net/listener/struct.UnixListener.html\" title=\"struct std::os::unix::net::listener::UnixListener\">UnixListener</a>&gt; for <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/unix/net/listener.rs.html#350\">source</a><a href=\"#method.from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html#tymethod.from\" class=\"fn\">from</a>(listener: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/os/unix/net/listener/struct.UnixListener.html\" title=\"struct std::os::unix::net::listener::UnixListener\">UnixListener</a>) -&gt; <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h4></section></summary><div class='docblock'>Converts to this type from the input type.</div></details></div></details>","From<UnixListener>","io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-From%3CTcpListener%3E-for-OwnedFd\" class=\"impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.63.0\">1.63.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#340\">source</a></span><a href=\"#impl-From%3CTcpListener%3E-for-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/net/tcp/struct.TcpListener.html\" title=\"struct std::net::tcp::TcpListener\">TcpListener</a>&gt; for <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#342\">source</a><a href=\"#method.from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.77.0/core/convert/trait.From.html#tymethod.from\" class=\"fn\">from</a>(tcp_listener: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.77.0/std/net/tcp/struct.TcpListener.html\" title=\"struct std::net::tcp::TcpListener\">TcpListener</a>) -&gt; <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h4></section></summary><div class='docblock'>Converts to this type from the input type.</div></details></div></details>","From<TcpListener>","io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-IntoRawFd-for-OwnedFd\" class=\"impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.63.0\">1.63.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#144\">source</a></span><a href=\"#impl-IntoRawFd-for-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.77.0/std/os/fd/raw/trait.IntoRawFd.html\" title=\"trait std::os::fd::raw::IntoRawFd\">IntoRawFd</a> for <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.into_raw_fd\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#146\">source</a><a href=\"#method.into_raw_fd\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.77.0/std/os/fd/raw/trait.IntoRawFd.html#tymethod.into_raw_fd\" class=\"fn\">into_raw_fd</a>(self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.77.0/std/primitive.i32.html\">i32</a></h4></section></summary><div class='docblock'>Consumes this object, returning the raw underlying file descriptor. <a href=\"https://doc.rust-lang.org/1.77.0/std/os/fd/raw/trait.IntoRawFd.html#tymethod.into_raw_fd\">Read more</a></div></details></div></details>","IntoRawFd","io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-IsTerminal-for-OwnedFd\" class=\"impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.70.0\">1.70.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#218\">source</a></span><a href=\"#impl-IsTerminal-for-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.77.0/std/io/stdio/trait.IsTerminal.html\" title=\"trait std::io::stdio::IsTerminal\">IsTerminal</a> for <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.is_terminal\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#218\">source</a><a href=\"#method.is_terminal\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.77.0/std/io/stdio/trait.IsTerminal.html#tymethod.is_terminal\" class=\"fn\">is_terminal</a>(&amp;self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.77.0/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Returns <code>true</code> if the descriptor/handle refers to a terminal/tty. <a href=\"https://doc.rust-lang.org/1.77.0/std/io/stdio/trait.IsTerminal.html#tymethod.is_terminal\">Read more</a></div></details></div></details>","IsTerminal","io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-AsRawFd-for-OwnedFd\" class=\"impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.63.0\">1.63.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#136\">source</a></span><a href=\"#impl-AsRawFd-for-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.77.0/std/os/fd/raw/trait.AsRawFd.html\" title=\"trait std::os::fd::raw::AsRawFd\">AsRawFd</a> for <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.as_raw_fd\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"https://doc.rust-lang.org/1.77.0/src/std/os/fd/owned.rs.html#138\">source</a><a href=\"#method.as_raw_fd\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.77.0/std/os/fd/raw/trait.AsRawFd.html#tymethod.as_raw_fd\" class=\"fn\">as_raw_fd</a>(&amp;self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.77.0/std/primitive.i32.html\">i32</a></h4></section></summary><div class='docblock'>Extracts the raw file descriptor. <a href=\"https://doc.rust-lang.org/1.77.0/std/os/fd/raw/trait.AsRawFd.html#tymethod.as_raw_fd\">Read more</a></div></details></div></details>","AsRawFd","io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"],["<section id=\"impl-FilelikeViewType-for-OwnedFd\" class=\"impl\"><a class=\"src rightside\" href=\"src/io_lifetimes/views.rs.html#211\">source</a><a href=\"#impl-FilelikeViewType-for-OwnedFd\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"io_lifetimes/views/trait.FilelikeViewType.html\" title=\"trait io_lifetimes::views::FilelikeViewType\">FilelikeViewType</a> for <a class=\"struct\" href=\"io_lifetimes/struct.OwnedFd.html\" title=\"struct io_lifetimes::OwnedFd\">OwnedFd</a></h3></section>","FilelikeViewType","io_lifetimes::portability::OwnedFilelike","io_lifetimes::portability::OwnedSocketlike"]]
};if (window.register_type_impls) {window.register_type_impls(type_impls);} else {window.pending_type_impls = type_impls;}})()