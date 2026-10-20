#!/usr/bin/perl
# Hosting probe (Perl CGI): prints what the CGI environment looks like from the
# inside, so a shared-hosting Apache setup (MPM, mod_cgi vs mod_cgid, suexec,
# kernel, CPU) can be reproduced locally. Read-only, core modules only, one
# request is enough. Deploy into cgi-bin (chmod 755) and open it once.
use strict;
use warnings;

$| = 1;
print "Content-Type: text/plain; charset=utf-8\r\n";
print "Cache-Control: no-store\r\n\r\n";

sub section { print "\n== $_[0] ==\n" }

sub slurp {
    my ($path) = @_;
    open(my $fh, '<', $path) or return undef;
    local $/;
    my $data = <$fh>;
    close $fh;
    return $data;
}

sub fd_kind {
    my ($fh) = @_;
    return 'pipe'   if -p $fh;
    return 'socket' if -S $fh;
    return 'tty'    if -t $fh;
    return 'file'   if -f $fh;
    return 'chardev' if -c $fh;
    return 'unknown';
}

sub proc_field {
    my ($pid, $field) = @_;
    my $status = slurp("/proc/$pid/status") or return undef;
    return undef unless $status =~ /^\Q$field\E:\s*(.*)$/m;
    my $value = $1;
    $value =~ s/\s+/ /g;
    return $value;
}

sub proc_cmdline {
    my ($pid) = @_;
    my $raw = slurp("/proc/$pid/cmdline");
    return undef unless defined $raw;
    $raw =~ s/\0/ /g;
    $raw =~ s/\s+$//;
    return $raw;
}

sub proc_ppid {
    my ($pid) = @_;
    my $stat = slurp("/proc/$pid/stat") or return undef;
    # "pid (comm) state ppid ..." — comm may contain spaces/parens, split after last ')'.
    my ($rest) = $stat =~ /\)\s+(.*)$/s;
    return undef unless defined $rest;
    my @f = split /\s+/, $rest;
    return $f[1];
}

section('probe');
print "kind: perl $]\n";
print "time: " . scalar(gmtime()) . " UTC\n";

section('process');
print "pid: $$\n";
print "ppid: " . getppid() . "\n";
print "uid: $< euid: $> gid: $( egid: $)\n";
print "user: " . (getpwuid($<) // 'n/a') . " euser: " . (getpwuid($>) // 'n/a') . "\n";
print "cwd: " . (slurp('/proc/self/cwd') // (eval { require Cwd; Cwd::getcwd() } // 'n/a')) . "\n";
print "exe: " . (readlink('/proc/self/exe') // 'n/a') . "\n";
print "stdin: "  . fd_kind(\*STDIN)  . "\n";
print "stdout: " . fd_kind(\*STDOUT) . "   (pipe => mod_cgi/prefork-style; socket => mod_cgid/threaded MPM)\n";
print "stderr: " . fd_kind(\*STDERR) . "\n";
print "threads: " . (proc_field($$, 'Threads') // 'n/a') . "\n";

section('parent chain (up to 6 levels)');
my $pid = getppid();
for my $level (1 .. 6) {
    last unless defined $pid && $pid > 0;
    my $comm = slurp("/proc/$pid/comm");
    chomp $comm if defined $comm;
    printf "%d: pid=%s comm=%s uid=%s threads=%s cmdline=%s\n",
        $level, $pid,
        $comm // 'n/a',
        proc_field($pid, 'Uid') // 'n/a',
        proc_field($pid, 'Threads') // 'n/a',
        proc_cmdline($pid) // 'n/a';
    last if $pid == 1;
    $pid = proc_ppid($pid);
}

section('httpd-like processes visible in /proc');
my ($nproc, $nthreads, $visible) = (0, 0, 0);
my %by_comm;
if (opendir(my $dh, '/proc')) {
    for my $entry (readdir $dh) {
        next unless $entry =~ /^\d+$/;
        my $comm = slurp("/proc/$entry/comm");
        next unless defined $comm;
        chomp $comm;
        $visible++;
        next unless $comm =~ /^(apache2|httpd|suexec|cgi|fcgi|php|lsphp|litespeed|nginx)/i;
        $by_comm{$comm}++;
        my $t = proc_field($entry, 'Threads');
        $nproc++;
        $nthreads += $t if defined $t;
    }
    closedir $dh;
}
print "visible processes in /proc: $visible (only own processes visible => hidepid is active)\n";
print "matching processes: $nproc, summed threads: $nthreads\n";
printf "  %-20s %d\n", $_, $by_comm{$_} for sort keys %by_comm;
print "hint: many 1-thread apache2/httpd => prefork; few procs with many threads => worker/event\n";

section('system');
print "kernel: " . (slurp('/proc/version') // "n/a\n");
my $os = slurp('/etc/os-release') // '';
print "os: " . (($os =~ /^PRETTY_NAME="?(.*?)"?$/m) ? $1 : 'n/a') . "\n";
my $cpuinfo = slurp('/proc/cpuinfo') // '';
my $ncpu = () = $cpuinfo =~ /^processor\s*:/mg;
my ($model) = $cpuinfo =~ /^model name\s*:\s*(.*)$/m;
print "cpus: $ncpu model: " . ($model // 'n/a') . "\n";
my $mem = slurp('/proc/meminfo') // '';
print "memtotal: " . (($mem =~ /^MemTotal:\s*(.*)$/m) ? $1 : 'n/a') . "\n";
print "loadavg: " . (slurp('/proc/loadavg') // "n/a\n");
print "proc/1 readable: " . (defined slurp('/proc/1/status') ? 'yes' : 'no') . "\n";
my $limits = slurp('/proc/self/limits') // '';
print "max processes (limits): " . ($limits =~ /^Max processes\s+(\S+)/m ? $1 : 'n/a') . "\n";

section('environment');
for my $key (sort keys %ENV) {
    my $value = $ENV{$key};
    $value =~ s/[\r\n]/ /g;
    print "$key=$value\n";
}
