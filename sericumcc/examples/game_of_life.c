#define W 40
#define H 30
#define RAND_MAX 2147483647

int usleep(int);
int putchar(char);
int rand();
int srand(int);
int puts(char *);

int show(int univ[H][W]) {
  for (int y = 0; y < H; y += 1) {
    for (int x = 0; x < W; x += 1) {
      if (univ[y][x] == 1)
        putchar('#');
      else
        putchar(' ');
    }
    puts("");
  }
  puts("");
  return 0;
}

int evolve(int univ[H][W]) {
  int new[H][W];

  for (int y = 0; y < H; y += 1) {
    for (int x = 0; x < W; x += 1) {
      int n = 0;
      for (int y1 = y - 1; y1 <= y + 1; y1+=1)
        for (int x1 = x - 1; x1 <= x + 1; x1+=1)
          if (univ[(y1 + H) % H][(x1 + W) % W] == 1)
            n += 1;
      if (univ[y][x] == 1) n -= 1;
      new[y][x] = 0;
      if (n == 3) {
        new[y][x] = 1;
      } else {
        if (n == 2) if (univ[y][x] == 1) new[y][x] = 1;
      }
    }
  }

  for (int y = 0; y < H; y += 1) 
    for (int x = 0; x < W; x += 1) 
      univ[y][x] = new[y][x];

  return 0;
}

int game() {
  int univ[H][W];
  for (int x = 0; x < W; x += 1) 
    for (int y = 0; y < H; y += 1)
      univ[y][x] = rand() < RAND_MAX / 10 ? 1 : 0;
  for (;;) {
    show(univ);
    evolve(univ);
    usleep(100000);
  }
  return 0;
}

int main() {
  game();
  return 0;
}
