import math
N=16
T=(14.0,1.0); B=(1.0,14.0)
NX,NY=-1/math.sqrt(2),-1/math.sqrt(2)  # towards the limb (upper left)

def pt(t,depth,tip=(T,B)):
    (tx,ty),(bx,by)=tip
    return (tx+(bx-tx)*t+NX*depth, ty+(by-ty)*t+NY*depth)

def new():
    return [['.']*N for _ in range(N)]

def put(g,x,y,c,over=True):
    x,y=int(round(x)),int(round(y))
    if 0<=x<N and 0<=y<N and (over or g[y][x]=='.'):
        g[y][x]=c

def limb(g,depth,width,tip=(T,B),shade=True):
    """Stamp the limb: depth(t), width(t) in pixels; outer side light, inner dark."""
    steps=400
    for i in range(steps+1):
        t=i/steps
        d=depth(t); w=width(t)
        for k in range(-8,9):
            o=k*0.25
            if abs(o)>w/2: continue
            x,y=pt(t,d+o,tip)
            c='3'
            if shade:
                if o> w/2-0.6 and w>1.3: c='4'
                elif o< -w/2+0.6 and w>1.3: c='2'
            put(g,x,y,c)

def string(g,tip=(T,B)):
    (tx,ty),(bx,by)=tip
    n=int(max(abs(bx-tx),abs(by-ty)))
    for i in range(1,n):
        t=i/n
        put(g,tx+(bx-tx)*t,ty+(by-ty)*t,'a',over=False)

def grip(g,c='1',t0=0.44,t1=0.56,depth=None,width=2):
    for i in range(41):
        t=t0+(t1-t0)*i/40
        d=depth(t)
        for o in (-width/2+0.3,0,width/2-0.3):
            x,y=pt(t,d+o); put(g,x,y,c)

def rows(g): return [''.join(r) for r in g]

BOW_SHAPES = {}
S = BOW_SHAPES
arc=lambda D:(lambda t:D*math.sin(math.pi*t))
const=lambda w:(lambda t:w)

# copper: the plain bow, a little sturdier, banded grip
g=new(); d=arc(4.6); limb(g,d,const(1.8)); grip(g,'1',depth=d); string(g)
for t in (0.2,0.8): x,y=pt(t,d(t)); put(g,x,y,'5')
S['copper']=rows(g)

# silver: recurve, the tips curl back forward
g=new()
d=lambda t:4.2*math.sin(math.pi*t)-2.2*(math.exp(-(t/0.09)**2)+math.exp(-((1-t)/0.09)**2))
limb(g,d,lambda t:1.2+0.8*math.sin(math.pi*t)); grip(g,'1',depth=d); string(g,( pt(0.04,d(0.04)), pt(0.96,d(0.96)) ))
S['silver']=rows(g)

# platinum: long flat longbow, past the corners, thin tips with nocks
tipL=((15.3,-0.3),(-0.3,15.3))
g=new(); d=arc(3.4); limb(g,d,lambda t:0.9+1.1*math.sin(math.pi*t),tip=tipL); string(g,tipL)
for i in range(41):
    t=0.44+0.12*i/40; x,y=pt(t,d(t),tipL); put(g,x,y,'1')
put(g,15,0,'5'); put(g,0,15,'5')
S['platinum']=rows(g)

# umbral: thorny dark bow, spikes on the back
g=new(); d=arc(4.4); limb(g,d,const(1.6)); string(g)
for t in (0.22,0.38,0.62,0.78):
    for k,c in ((1.4,'2'),(2.1,'2'),(2.8,'1')):
        x,y=pt(t+(0.02 if t<0.5 else -0.02)*k,d(t)+k); put(g,x,y,c)
grip(g,'1',depth=d)
S['umbral']=rows(g)

# sanguinite: thick, fleshy, an eye at the grip
g=new(); d=arc(4.4); limb(g,d,lambda t:1.0+2.2*math.sin(math.pi*t)**1.5); string(g)
x,y=pt(0.5,d(0.5)); put(g,x,y,'5'); x,y=pt(0.5,d(0.5)-0.8); put(g,x,y,'1')
S['sanguinite']=rows(g)

# magmarite: angular, burning tips
g=new(); d=lambda t:5.2*(1-abs(2*t-1))**0.7; limb(g,d,const(2.0)); string(g)
for t,dd in ((0.02,1.5),(0.06,2.5),(0.98,1.5),(0.94,2.5)):
    x,y=pt(t,d(t)+dd); put(g,x,y,'5')
grip(g,'1',depth=d)
S['magmarite']=rows(g)

# cobalt: compound bow, straight limbs, cams at the tips
g=new(); d=lambda t:4.2*(1-abs(2*t-1)); limb(g,d,const(1.5)); string(g,(pt(0.12,d(0.12)),pt(0.88,d(0.88))))
for t in (0.12,0.88):
    cx,cy=pt(t,d(t)+0.3)
    for dx in (-1,0,1):
        for dy in (-1,0,1):
            put(g,cx+dx,cy+dy,'4' if dx+dy<0 else '2')
    put(g,cx,cy,'5')
grip(g,'1',depth=d)
S['cobalt']=rows(g)

# mythril: reflex, the grip set back between two humps, a gem
g=new(); d=lambda t:5.0*math.sin(math.pi*t)-2.2*math.exp(-((t-0.5)/0.11)**2); limb(g,d,const(1.6)); string(g)
x,y=pt(0.5,d(0.5)); put(g,x,y,'5'); x,y=pt(0.5,d(0.5)+1); put(g,x,y,'4')
S['mythril']=rows(g)

# adamantite: heavy, blade fins on the back of the limbs
g=new(); d=arc(4.2); limb(g,d,lambda t:1.6+1.0*math.sin(math.pi*t)); string(g)
for t in (0.25,0.75):
    s_ = 1 if t < 0.5 else -1
    for k in range(4):
        for j in range(4-k):
            x,y=pt(t+s_*0.035*(j-k*0.5),d(t)+1.8+k*0.75); put(g,x,y,'5' if k==3 else '4')
grip(g,'1',depth=d)
S['adamantite']=rows(g)

# radiant: winged, feathers off the back near the tips
g=new(); d=arc(4.0); limb(g,d,const(1.4)); string(g)
for t in (0.16,0.24,0.32,0.68,0.76,0.84):
    lean = 0.04 if t < 0.5 else -0.04
    for k in range(3):
        x,y=pt(t-lean*k,d(t)+1.0+k*0.9); put(g,x,y,'5' if k==2 else '4')
x,y=pt(0.5,d(0.5)); put(g,x,y,'1')
S['radiant']=rows(g)

# verdite: a living vine, leaves on the back
g=new(); d=arc(4.4); limb(g,d,const(1.1),shade=False); string(g)
for t in (0.2,0.4,0.6,0.8):
    x,y=pt(t,d(t)+1.3)
    for dx,dy,c in ((0,0,'4'),(-1,0,'4'),(0,-1,'5'),(-1,-1,'4')): put(g,x+dx,y+dy,c)
S['verdite']=rows(g)

# lumite: two thin arcs meeting at the tips, a star in the middle
g=new(); d1=arc(5.8); d2=arc(2.2); limb(g,d1,const(0.7),shade=False); limb(g,d2,const(0.7),shade=False)
for (x,y) in ((0,0),): pass
string(g)
x,y=pt(0.5,4.0)
for dx,dy in ((0,0),(1,0),(-1,0),(0,1),(0,-1)): put(g,x+dx,y+dy,'5' if (dx,dy)==(0,0) else '4')
S['lumite']=rows(g)

if __name__=='__main__':
    from PIL import Image
    P={'1':(50,40,30),'2':(110,80,50),'3':(170,120,70),'4':(220,170,110),'5':(255,240,200),'a':(230,225,210)}
    names=list(S)
    W=Image.new('RGB',(len(names)*136,136),(40,80,40))
    for i,n in enumerate(names):
        im=Image.new('RGB',(16,16),(40,80,40))
        for y,r in enumerate(S[n]):
            for x,c in enumerate(r):
                if c in P: im.putpixel((x,y),P[c])
        W.paste(im.resize((128,128),Image.NEAREST),(i*136,0))
    W.save('/tmp/claude-0/sp/bowd/shapes.png')
    print(names)
